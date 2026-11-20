use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use sysinfo::{Disks, Networks, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
use tauri::{AppHandle, Manager, Runtime};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessRow {
    pub pid: u32,
    pub parent: Option<u32>,
    pub name: String,
    pub cpu: f32,
    pub memory: u64,
    pub private_memory: Option<u64>,
    pub role: String,
}

#[derive(Clone)]
struct ProcessIdentity {
    row: ProcessRow,
    executable: Option<String>,
    started: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationRow {
    id: String,
    name: String,
    root_pid: u32,
    process_count: usize,
    cpu: f32,
    memory: u64,
    private_memory: Option<u64>,
    known_private_memory: u64,
    private_memory_known: usize,
    association: String,
    processes: Vec<ProcessRow>,
}

fn normalize_executable(path: &str) -> String {
    path.trim_start_matches(r"\\?\")
        .replace('/', r"\")
        .to_lowercase()
}

fn application_groups(
    identities: &[ProcessIdentity],
    services: &HashMap<u32, crate::infrastructure::process_resources::ServiceProcess>,
) -> Vec<ApplicationRow> {
    let by_pid: HashMap<_, _> = identities.iter().map(|p| (p.row.pid, p)).collect();
    let mut paths: HashMap<u32, String> = identities
        .iter()
        .filter_map(|p| {
            p.executable
                .as_ref()
                .map(|path| (p.row.pid, normalize_executable(path)))
        })
        .collect();
    let mut verified = HashMap::new();
    for (pid, service) in services {
        let path = normalize_executable(&service.executable);
        if by_pid.contains_key(pid) && paths.get(pid).map_or(true, |actual| actual == &path) {
            paths.insert(*pid, path.clone());
            verified.insert(*pid, path);
        }
    }
    let mut service_roots: HashMap<String, u32> = HashMap::new();
    for (pid, path) in &verified {
        service_roots
            .entry(path.clone())
            .and_modify(|old| *old = (*old).min(*pid))
            .or_insert(*pid);
    }
    let mut groups: HashMap<String, ApplicationRow> = HashMap::new();
    for identity in identities {
        let mut root = identity.row.pid;
        let path = paths.get(&root);
        let association;
        if let Some(service) = path.and_then(|path| service_roots.get(path)) {
            root = *service;
            association = "verified-service-executable";
        } else if let Some(path) = path {
            association = "same-executable-tree";
            let mut visited = HashSet::from([root]);
            while let Some(current) = by_pid.get(&root) {
                let Some(parent) = current.row.parent.and_then(|pid| by_pid.get(&pid)) else {
                    break;
                };
                if paths.get(&parent.row.pid) != Some(path) || parent.started > current.started {
                    break;
                }
                if !visited.insert(parent.row.pid) {
                    root = *visited.iter().min().unwrap_or(&root);
                    break;
                }
                root = parent.row.pid;
            }
        } else {
            association = "single-process";
        }
        let root_process = by_pid.get(&root).unwrap_or(&identity);
        let id = format!("{association}:{root}:{}", root_process.started);
        let mut row = identity.row.clone();
        row.role = if let Some(service) = services
            .get(&row.pid)
            .filter(|_| verified.contains_key(&row.pid))
        {
            format!("Windows 服务：{}", service.name)
        } else if association == "verified-service-executable" {
            "客户端".into()
        } else {
            "进程".into()
        };
        let group = groups.entry(id.clone()).or_insert_with(|| ApplicationRow {
            id,
            name: root_process.row.name.clone(),
            root_pid: root,
            process_count: 0,
            cpu: 0.0,
            memory: 0,
            private_memory: None,
            known_private_memory: 0,
            private_memory_known: 0,
            association: association.into(),
            processes: Vec::new(),
        });
        group.cpu = (group.cpu + row.cpu).min(100.0);
        group.memory = group.memory.saturating_add(row.memory);
        if let Some(value) = row.private_memory {
            group.known_private_memory = group.known_private_memory.saturating_add(value);
            group.private_memory_known += 1;
        }
        group.processes.push(row);
    }
    let mut groups: Vec<_> = groups.into_values().collect();
    for group in &mut groups {
        group.process_count = group.processes.len();
        if group.private_memory_known == group.process_count {
            group.private_memory = Some(group.known_private_memory);
        }
        group.processes.sort_by_key(|p| p.pid);
    }
    groups.sort_by(|a, b| {
        b.known_private_memory
            .cmp(&a.known_private_memory)
            .then_with(|| a.id.cmp(&b.id))
    });
    groups
}

fn external_process(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "umi-ocr.exe"
            | "everything.exe"
            | "llama-server.exe"
            | "paddleocr-json.exe"
            | "rapidocr-json.exe"
    )
}

pub fn descendants(rows: &[ProcessRow], roots: &HashSet<u32>) -> HashSet<u32> {
    let mut ids = roots.clone();
    loop {
        let old = ids.len();
        for row in rows {
            if row.parent.is_some_and(|parent| ids.contains(&parent)) {
                ids.insert(row.pid);
            }
        }
        if ids.len() == old {
            break;
        }
    }
    ids
}

fn owned_processes(rows: &[ProcessRow], host: u32, browser: Option<u32>) -> HashSet<u32> {
    let runtime_rows: Vec<_> = rows
        .iter()
        .filter(|row| row.name.eq_ignore_ascii_case("msedgewebview2.exe"))
        .cloned()
        .collect();
    let roots = HashSet::from([browser.unwrap_or(host)]);
    let mut owned = descendants(&runtime_rows, &roots);
    owned.insert(host);
    owned
}

struct Sampler {
    system: System,
    networks: Networks,
    disks: Disks,
    last: Option<Instant>,
    cached: Option<Value>,
    network_totals: HashMap<String, (u64, u64)>,
    disks_refreshed: Instant,
}
impl Sampler {
    fn new() -> Self {
        Self {
            system: System::new(),
            networks: Networks::new_with_refreshed_list(),
            disks: Disks::new_with_refreshed_list(),
            last: None,
            cached: None,
            network_totals: HashMap::new(),
            disks_refreshed: Instant::now(),
        }
    }
}
static SAMPLER: OnceLock<Mutex<Sampler>> = OnceLock::new();

fn browser_root<R: Runtime>(app: &AppHandle<R>) -> Option<u32> {
    #[cfg(windows)]
    {
        let window = app.get_webview_window("main")?;
        let (send, recv) = std::sync::mpsc::channel();
        window
            .with_webview(move |view| {
                let id = unsafe {
                    view.controller().CoreWebView2().ok().and_then(|core| {
                        let mut id = 0;
                        core.BrowserProcessId(&mut id).ok().map(|_| id)
                    })
                };
                let _ = send.send(id);
            })
            .ok()?;
        recv.recv_timeout(Duration::from_millis(300))
            .ok()
            .flatten()
            .filter(|pid| *pid != 0)
    }
    #[cfg(not(windows))]
    {
        let _ = app;
        None
    }
}

pub fn snapshot<R: Runtime>(app: &AppHandle<R>) -> Result<Value, String> {
    let mut state = SAMPLER
        .get_or_init(|| Mutex::new(Sampler::new()))
        .lock()
        .map_err(|e| e.to_string())?;
    if state
        .last
        .is_some_and(|time| time.elapsed() < Duration::from_secs(1))
    {
        return state
            .cached
            .clone()
            .ok_or_else(|| "资源采样尚未完成".into());
    }
    let elapsed = state
        .last
        .map(|time| time.elapsed().as_secs_f64())
        .unwrap_or(0.0);
    let warming = elapsed == 0.0 || elapsed > 10.0;
    state.system.refresh_memory();
    if state.last.is_none() {
        state.system.refresh_cpu_all();
    } else {
        state.system.refresh_cpu_usage();
    }
    state.system.refresh_processes_specifics(
        ProcessesToUpdate::All,
        ProcessRefreshKind::default()
            .with_cpu()
            .with_memory()
            .with_exe(UpdateKind::OnlyIfNotSet),
    );
    state.networks.refresh_list();
    state.networks.refresh();
    if state.disks_refreshed.elapsed() >= Duration::from_secs(30) {
        state.disks.refresh_list();
        state.disks.refresh();
        state.disks_refreshed = Instant::now();
    }
    let cores = state.system.cpus().len().max(1);
    let identities: Vec<ProcessIdentity> = state
        .system
        .processes()
        .iter()
        .map(|(pid, p)| ProcessIdentity {
            row: ProcessRow {
                pid: pid.as_u32(),
                parent: p.parent().map(|x| x.as_u32()),
                name: p.name().to_string_lossy().into_owned(),
                cpu: if warming {
                    0.0
                } else {
                    (p.cpu_usage() / cores as f32).clamp(0.0, 100.0)
                },
                memory: p.memory(),
                private_memory: crate::infrastructure::process_resources::private_working_set(
                    pid.as_u32(),
                ),
                role: "进程".into(),
            },
            executable: p.exe().map(|path| path.to_string_lossy().into_owned()),
            started: p.start_time(),
        })
        .collect();
    let service_pids = identities
        .iter()
        .filter(|p| external_process(&p.row.name))
        .map(|p| p.row.pid)
        .collect();
    let services = crate::infrastructure::process_resources::service_processes(&service_pids);
    let applications = application_groups(&identities, &services);
    let rows: Vec<_> = applications
        .iter()
        .flat_map(|group| group.processes.iter().cloned())
        .collect();
    let external_identities: Vec<_> = identities
        .iter()
        .filter(|p| external_process(&p.row.name))
        .cloned()
        .collect();
    let external_applications = application_groups(&external_identities, &services);
    let browser = browser_root(app);
    let owned = owned_processes(&rows, std::process::id(), browser);
    let app_rows: Vec<_> = rows
        .iter()
        .filter(|r| owned.contains(&r.pid))
        .cloned()
        .collect();
    let runtime_rows: Vec<_> = app_rows
        .iter()
        .filter(|r| r.name.eq_ignore_ascii_case("msedgewebview2.exe"))
        .cloned()
        .collect();
    let memory: u64 = app_rows.iter().map(|r| r.memory).sum();
    let cpu: f32 = app_rows.iter().map(|r| r.cpu).sum();
    let mut top_memory = rows.clone();
    top_memory.sort_by_key(|r| std::cmp::Reverse(r.memory));
    top_memory.truncate(20);
    let mut top_cpu = rows.clone();
    top_cpu.sort_by(|a, b| b.cpu.total_cmp(&a.cpu));
    top_cpu.truncate(20);
    let external: Vec<_> = rows
        .iter()
        .filter(|r| external_process(&r.name))
        .cloned()
        .collect();
    let totals: HashMap<String, (u64, u64)> = state
        .networks
        .iter()
        .map(|(name, n)| (name.clone(), (n.total_received(), n.total_transmitted())))
        .collect();
    let network:Vec<_>=totals.iter().map(|(name,(received,sent))|{let previous=state.network_totals.get(name);json!({"name":name,"receivedPerSecond":if warming{0.0}else{previous.map(|old|received.saturating_sub(old.0) as f64/elapsed).unwrap_or(0.0)},"transmittedPerSecond":if warming{0.0}else{previous.map(|old|sent.saturating_sub(old.1) as f64/elapsed).unwrap_or(0.0)}})}).collect();
    state.network_totals = totals;
    let disks:Vec<_>=state.disks.iter().map(|d|json!({"name":d.name().to_string_lossy(),"mount":d.mount_point().to_string_lossy(),"total":d.total_space(),"available":d.available_space()})).collect();
    let value = json!({"version":1,"sampledAt":SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_millis(),"warming":warming,
  "machine":{"cpu":if warming{0.0}else{state.system.global_cpu_usage()},"cores":cores,"cpuName":state.system.cpus().first().map(|c|c.brand()).unwrap_or("未知"),"totalMemory":state.system.total_memory(),"usedMemory":state.system.used_memory(),"totalSwap":state.system.total_swap(),"usedSwap":state.system.used_swap(),"uptime":System::uptime(),"os":System::long_os_version(),"gpu":null,"temperature":null},
  "networks":network,"disks":disks,"topMemory":top_memory,"topCpu":top_cpu,"processDetailsVersion":2,"applications":applications,"externalApplications":external_applications,
  "mytools":{"pid":std::process::id(),"cpu":cpu.min(100.0),"memory":memory,"processes":app_rows,"webviewProcesses":runtime_rows,"browserPid":browser,"association":if browser.is_some(){"browser-and-descendants"}else{"parent-tree-only"}},"externalServices":external});
    state.last = Some(Instant::now());
    state.cached = Some(value.clone());
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn identity(
        pid: u32,
        parent: Option<u32>,
        exe: Option<&str>,
        started: u64,
        private: Option<u64>,
    ) -> ProcessIdentity {
        ProcessIdentity {
            row: ProcessRow {
                pid,
                parent,
                name: "App.exe".into(),
                cpu: 2.0,
                memory: 100,
                private_memory: private,
                role: "进程".into(),
            },
            executable: exe.map(str::to_string),
            started,
        }
    }
    #[test]
    fn groups_only_related_instances_with_verified_executables() {
        let rows = vec![
            identity(1, None, Some(r"C:\App.exe"), 1, Some(10)),
            identity(2, Some(1), Some(r"c:\app.EXE"), 2, Some(20)),
            identity(3, None, Some(r"C:\App.exe"), 3, Some(30)),
            identity(4, Some(1), Some(r"D:\App.exe"), 4, Some(40)),
            identity(5, Some(1), None, 5, Some(50)),
        ];
        let groups = application_groups(&rows, &HashMap::new());
        assert_eq!(groups.len(), 4);
        let group = groups.iter().find(|g| g.root_pid == 1).unwrap();
        assert_eq!(group.process_count, 2);
        assert_eq!(group.private_memory, Some(30));
        assert_eq!(group.memory, 200);
        assert_eq!(group.cpu, 4.0);
        let json = serde_json::to_string(&groups).unwrap();
        assert!(!json.contains("\"executable\":"));
        assert!(!json.contains(r"C:\\App.exe"));
    }
    #[test]
    fn groups_all_small_children_before_any_ranking_and_preserves_missing_values() {
        let mut rows = vec![identity(1, None, Some("app.exe"), 1, Some(100))];
        rows.extend((2..30).map(|id| identity(id, Some(1), Some("app.exe"), id as u64, None)));
        let groups = application_groups(&rows, &HashMap::new());
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].process_count, 29);
        assert_eq!(groups[0].memory, 2900);
        assert_eq!(groups[0].private_memory, None);
        assert_eq!(groups[0].private_memory_known, 1);
        assert_eq!(groups[0].known_private_memory, 100);
    }
    #[test]
    fn rejects_reused_parent_pid_and_terminates_cycles() {
        let rows = vec![
            identity(1, None, Some("app.exe"), 100, Some(1)),
            identity(2, Some(1), Some("app.exe"), 20, Some(1)),
            identity(3, Some(4), Some("app.exe"), 1, Some(1)),
            identity(4, Some(3), Some("app.exe"), 1, Some(1)),
        ];
        let groups = application_groups(&rows, &HashMap::new());
        assert_eq!(groups.len(), 3);
        assert_eq!(
            groups
                .iter()
                .find(|g| g.root_pid == 3)
                .unwrap()
                .process_count,
            2
        );
    }
    #[test]
    fn combines_verified_service_and_client_but_not_another_installation() {
        use crate::infrastructure::process_resources::ServiceProcess;
        let rows = vec![
            identity(1, None, None, 1, Some(1)),
            identity(2, None, Some(r"C:\Tools\Everything.exe"), 2, Some(2)),
            identity(3, None, Some(r"D:\Tools\Everything.exe"), 3, Some(3)),
        ];
        let services = HashMap::from([(
            1,
            ServiceProcess {
                name: "Everything".into(),
                executable: r"C:\Tools\Everything.exe".into(),
            },
        )]);
        let groups = application_groups(&rows, &services);
        assert_eq!(groups.len(), 2);
        let group = groups.iter().find(|g| g.root_pid == 1).unwrap();
        assert_eq!(group.process_count, 2);
        assert!(group.processes[0].role.contains("Windows 服务"));
        assert_eq!(group.processes[1].role, "客户端");
        let conflicting = HashMap::from([(
            3,
            ServiceProcess {
                name: "Stale".into(),
                executable: r"C:\Tools\Everything.exe".into(),
            },
        )]);
        assert_eq!(application_groups(&rows, &conflicting).len(), 3);
    }
    #[test]
    fn launched_apps_and_external_services_are_not_counted_as_mytools() {
        let row = |pid, parent, name: &str| ProcessRow {
            pid,
            parent,
            name: name.into(),
            cpu: 0.0,
            memory: 1,
            private_memory: Some(1),
            role: "进程".into(),
        };
        let rows = vec![
            row(1, None, "mytools.exe"),
            row(2, Some(1), "msedgewebview2.exe"),
            row(3, Some(2), "msedgewebview2.exe"),
            row(4, Some(1), "umi-ocr.exe"),
            row(5, Some(1), "other.exe"),
            row(6, Some(5), "msedgewebview2.exe"),
            row(7, None, "msedgewebview2.exe"),
        ];
        assert_eq!(owned_processes(&rows, 1, Some(2)), HashSet::from([1, 2, 3]));
        assert_eq!(owned_processes(&rows, 1, None), HashSet::from([1, 2, 3]));
    }
    #[test]
    fn process_tree_includes_only_owned_descendants_and_handles_cycles() {
        let row = |pid, parent| ProcessRow {
            pid,
            parent,
            name: "test".into(),
            cpu: 0.0,
            memory: 1,
            private_memory: Some(1),
            role: "进程".into(),
        };
        let rows = vec![
            row(1, None),
            row(2, Some(1)),
            row(3, Some(2)),
            row(4, Some(99)),
            row(5, Some(6)),
            row(6, Some(5)),
        ];
        assert_eq!(
            descendants(&rows, &HashSet::from([1])),
            HashSet::from([1, 2, 3])
        );
        assert_eq!(
            descendants(&rows, &HashSet::from([5])),
            HashSet::from([5, 6])
        );
    }
}
