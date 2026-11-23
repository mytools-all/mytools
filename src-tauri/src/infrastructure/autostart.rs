#[cfg(windows)]
mod platform {
    use std::path::Path;
    use std::sync::Mutex;
    use windows::core::{Interface, BSTR, VARIANT};
    use windows::Win32::Foundation::VARIANT_BOOL;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
        COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::System::TaskScheduler::{
        IExecAction, IRegisteredTask, ITaskDefinition, ITaskFolder, ITaskService, TaskScheduler,
        TASK_CREATE_OR_UPDATE, TASK_IGNORE_REGISTRATION_TRIGGERS, TASK_LOGON_INTERACTIVE_TOKEN,
        TASK_RUNLEVEL_LUA, TASK_TRIGGER_LOGON,
    };

    static LOCK: Mutex<()> = Mutex::new(());
    struct Apartment(bool);
    impl Apartment {
        fn new() -> Result<Self, String> {
            let result = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
            if result.is_err() && result.0 != 0x8001_010Eu32 as i32 {
                return Err(format!("初始化 Windows 任务计划失败：{result}"));
            }
            Ok(Self(result.is_ok()))
        }
    }
    impl Drop for Apartment {
        fn drop(&mut self) {
            if self.0 {
                unsafe { CoUninitialize() };
            }
        }
    }

    struct Scheduler {
        service: ITaskService,
        folder: ITaskFolder,
        user: String,
        name: String,
        _apartment: Apartment,
    }
    impl Scheduler {
        fn connect() -> Result<Self, String> {
            let apartment = Apartment::new()?;
            let service: ITaskService =
                unsafe { CoCreateInstance(&TaskScheduler, None, CLSCTX_INPROC_SERVER) }
                    .map_err(task_error)?;
            let empty = VARIANT::default();
            unsafe { service.Connect(&empty, &empty, &empty, &empty) }.map_err(task_error)?;
            let user = unsafe { service.ConnectedUser() }
                .map_err(task_error)?
                .to_string();
            let domain = unsafe { service.ConnectedDomain() }
                .map_err(task_error)?
                .to_string();
            let user = if domain.is_empty() {
                user
            } else {
                format!("{domain}\\{user}")
            };
            let folder = unsafe { service.GetFolder(&BSTR::from("\\")) }.map_err(task_error)?;
            let name = task_name(&user);
            Ok(Self {
                service,
                folder,
                user,
                name,
                _apartment: apartment,
            })
        }
        fn task(&self) -> Result<Option<IRegisteredTask>, String> {
            match unsafe { self.folder.GetTask(&BSTR::from(&self.name)) } {
                Ok(task) => {
                    let definition = unsafe { task.Definition() }.map_err(task_error)?;
                    let info = unsafe { definition.RegistrationInfo() }.map_err(task_error)?;
                    let mut source = BSTR::default();
                    unsafe { info.Source(&mut source) }.map_err(task_error)?;
                    if source != marker(&self.user) {
                        return Err(
                            "存在同名的其他计划任务，请先在 Windows 任务计划程序中检查，未作改动"
                                .into(),
                        );
                    }
                    Ok(Some(task))
                }
                Err(error) if matches!(error.code().0 as u32, 0x8007_0002 | 0x8007_0003) => {
                    Ok(None)
                }
                Err(error) => Err(task_error(error)),
            }
        }
        fn enabled(&self, executable: &Path) -> Result<bool, String> {
            let Some(task) = self.task()? else {
                return Ok(false);
            };
            if unsafe { task.Enabled() }.map_err(task_error)?.0 == 0 {
                return Ok(false);
            }
            definition_matches(
                &unsafe { task.Definition() }.map_err(task_error)?,
                executable,
            )
        }
        fn set(&self, enabled: bool, executable: &Path) -> Result<bool, String> {
            let existing = self.task()?;
            if !enabled {
                if existing.is_some() {
                    unsafe { self.folder.DeleteTask(&BSTR::from(&self.name), 0) }
                        .map_err(task_error)?;
                }
                return Ok(false);
            }
            let definition = unsafe { self.service.NewTask(0) }.map_err(task_error)?;
            unsafe { definition.SetXmlText(&BSTR::from(task_xml(&self.user, executable)?)) }
                .map_err(task_error)?;
            let empty = VARIANT::default();
            let user = VARIANT::from(self.user.as_str());
            unsafe {
                self.folder.RegisterTaskDefinition(
                    &BSTR::from(&self.name),
                    &definition,
                    TASK_CREATE_OR_UPDATE.0 | TASK_IGNORE_REGISTRATION_TRIGGERS.0,
                    &user,
                    &empty,
                    TASK_LOGON_INTERACTIVE_TOKEN,
                    &empty,
                )
            }
            .map_err(task_error)?;
            self.enabled(executable)
        }
    }

    fn task_error(error: windows::core::Error) -> String {
        format!("Windows 任务计划操作失败：{error}")
    }
    fn task_name(user: &str) -> String {
        let hash = user
            .to_lowercase()
            .bytes()
            .fold(0xcbf29ce484222325u64, |hash, byte| {
                (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
            });
        format!("MyTools-Autostart-{hash:016x}")
    }
    fn marker(user: &str) -> String {
        format!("MyTools/autostart/v1/{}", user.to_lowercase())
    }
    fn xml_escape(value: &str) -> String {
        value
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }
    fn task_xml(user: &str, executable: &Path) -> Result<String, String> {
        let directory = executable.parent().ok_or("程序路径无效")?;
        let executable = executable.to_str().ok_or("程序路径无法转换为 Unicode")?;
        let directory = directory.to_str().ok_or("程序目录无法转换为 Unicode")?;
        Ok(format!(
            r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo><Description>MyTools 当前用户登录启动</Description><Source>{source}</Source></RegistrationInfo>
  <Triggers><LogonTrigger><Enabled>true</Enabled><UserId>{user}</UserId><Delay>PT5S</Delay></LogonTrigger></Triggers>
  <Principals><Principal id="CurrentUser"><UserId>{user}</UserId><LogonType>InteractiveToken</LogonType><RunLevel>LeastPrivilege</RunLevel></Principal></Principals>
  <Settings><MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy><DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries><StopIfGoingOnBatteries>false</StopIfGoingOnBatteries><StartWhenAvailable>true</StartWhenAvailable><Enabled>true</Enabled><ExecutionTimeLimit>PT0S</ExecutionTimeLimit></Settings>
  <Actions Context="CurrentUser"><Exec><Command>{executable}</Command><Arguments>--autostart</Arguments><WorkingDirectory>{directory}</WorkingDirectory></Exec></Actions>
</Task>"#,
            source = xml_escape(&marker(user)),
            user = xml_escape(user),
            executable = xml_escape(executable),
            directory = xml_escape(directory)
        ))
    }
    fn definition_matches(definition: &ITaskDefinition, executable: &Path) -> Result<bool, String> {
        let principal = unsafe { definition.Principal() }.map_err(task_error)?;
        let mut level = TASK_RUNLEVEL_LUA;
        let mut logon = TASK_LOGON_INTERACTIVE_TOKEN;
        unsafe { principal.RunLevel(&mut level) }.map_err(task_error)?;
        unsafe { principal.LogonType(&mut logon) }.map_err(task_error)?;
        if level != TASK_RUNLEVEL_LUA || logon != TASK_LOGON_INTERACTIVE_TOKEN {
            return Ok(false);
        }
        let actions = unsafe { definition.Actions() }.map_err(task_error)?;
        let mut count = 0;
        unsafe { actions.Count(&mut count) }.map_err(task_error)?;
        if count != 1 {
            return Ok(false);
        }
        let action: IExecAction = unsafe { actions.get_Item(1) }
            .map_err(task_error)?
            .cast()
            .map_err(task_error)?;
        let mut path = BSTR::default();
        let mut arguments = BSTR::default();
        unsafe { action.Path(&mut path) }.map_err(task_error)?;
        unsafe { action.Arguments(&mut arguments) }.map_err(task_error)?;
        let normalize = |path: &Path| {
            path.to_string_lossy()
                .replace('/', "\\")
                .trim_start_matches(r"\\?\")
                .to_lowercase()
        };
        if normalize(Path::new(&path.to_string())) != normalize(executable)
            || arguments != "--autostart"
        {
            return Ok(false);
        }
        let triggers = unsafe { definition.Triggers() }.map_err(task_error)?;
        unsafe { triggers.Count(&mut count) }.map_err(task_error)?;
        if count != 1 {
            return Ok(false);
        }
        let trigger = unsafe { triggers.get_Item(1) }.map_err(task_error)?;
        let mut kind = TASK_TRIGGER_LOGON;
        let mut enabled = VARIANT_BOOL::default();
        unsafe { trigger.Type(&mut kind) }.map_err(task_error)?;
        unsafe { trigger.Enabled(&mut enabled) }.map_err(task_error)?;
        Ok(kind == TASK_TRIGGER_LOGON && enabled.0 != 0)
    }
    pub fn is_enabled() -> Result<bool, String> {
        let _guard = LOCK.lock().map_err(|e| e.to_string())?;
        let executable = std::env::current_exe().map_err(|e| e.to_string())?;
        Scheduler::connect()?.enabled(&executable)
    }
    pub fn set_enabled(enabled: bool) -> Result<bool, String> {
        let _guard = LOCK.lock().map_err(|e| e.to_string())?;
        let executable = std::env::current_exe().map_err(|e| e.to_string())?;
        Scheduler::connect()?.set(enabled, &executable)
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn task_xml_preserves_unicode_and_escapes_special_characters() {
            let _apartment = Apartment::new().expect("COM");
            let service: ITaskService =
                unsafe { CoCreateInstance(&TaskScheduler, None, CLSCTX_INPROC_SERVER) }
                    .expect("service");
            let definition = unsafe { service.NewTask(0) }.expect("definition");
            let path = Path::new(r"C:\中文 & tools\mytools.exe");
            let xml = task_xml(r"Computer\Alice & Bob", path).expect("xml");
            assert!(xml.contains("Alice &amp; Bob"));
            unsafe { definition.SetXmlText(&BSTR::from(xml)) }.expect("valid task XML");
            assert!(definition_matches(&definition, path).expect("matches"));
            assert!(
                !definition_matches(&definition, Path::new(r"C:\other.exe")).expect("other path")
            );
            let trigger =
                unsafe { definition.Triggers().expect("triggers").get_Item(1) }.expect("trigger");
            unsafe { trigger.SetEnabled(VARIANT_BOOL(0)) }.expect("disable trigger");
            assert!(!definition_matches(&definition, path).expect("disabled trigger"));
        }
        #[test]
        fn task_identity_is_stable_and_separated_by_user() {
            assert_eq!(task_name(r"Computer\Alice"), task_name(r"COMPUTER\ALICE"));
            assert_ne!(task_name(r"Computer\Alice"), task_name(r"Computer\Bob"));
            assert_eq!(xml_escape("<&>\"'"), "&lt;&amp;&gt;&quot;&apos;");
        }
    }
}
#[cfg(windows)]
pub use platform::{is_enabled, set_enabled};
