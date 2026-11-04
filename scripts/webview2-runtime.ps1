function Get-WebView2RuntimeVersion {
    param([Parameter(Mandatory)][string]$LoaderPath)
    if (-not ("MyTools.E2E.WebViewLoader" -as [type])) {
        Add-Type @'
using System;
using System.Runtime.InteropServices;
namespace MyTools.E2E {
    public static class WebViewLoader {
        [UnmanagedFunctionPointer(CallingConvention.StdCall, CharSet = CharSet.Unicode)]
        private delegate int GetVersion([MarshalAs(UnmanagedType.LPWStr)] string folder, out IntPtr version);
        public static string Read(string path) {
            IntPtr module = NativeLibrary.Load(path);
            try {
                var getVersion = Marshal.GetDelegateForFunctionPointer<GetVersion>(NativeLibrary.GetExport(module, "GetAvailableCoreWebView2BrowserVersionString"));
                IntPtr pointer;
                int result = getVersion(null, out pointer);
                if (result < 0) Marshal.ThrowExceptionForHR(result);
                try { return Marshal.PtrToStringUni(pointer); }
                finally { Marshal.FreeCoTaskMem(pointer); }
            } finally { NativeLibrary.Free(module); }
        }
    }
}
'@
    }
    $taskVersion = [MyTools.E2E.WebViewLoader]::Read((Resolve-Path -LiteralPath $LoaderPath).Path)
    if ($taskVersion -notmatch '^(\d+\.\d+\.\d+\.\d+)') { throw "Cannot determine WebView2 Runtime version: $taskVersion" }
    return $Matches[1]
}

function Test-WebViewDriverVersion {
    param([Parameter(Mandatory)][string]$RuntimeVersion, [Parameter(Mandatory)][string]$DriverVersion)
    if ($RuntimeVersion -notmatch '^\d+\.\d+\.\d+\.\d+$' -or $DriverVersion -notmatch '^\d+\.\d+\.\d+\.\d+$') { return $false }
    return (($RuntimeVersion.Split('.')[0..2] -join '.') -eq ($DriverVersion.Split('.')[0..2] -join '.'))
}
