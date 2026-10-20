[CmdletBinding()]
param([ValidateSet('French','English')][string]$Language='French',[switch]$Light,[switch]$List,[ValidateSet('Playing','Early','Minimized')][string]$CloseScenario='Playing')
$ErrorActionPreference = 'Stop'
# Native QA uses only generated fixtures and an isolated RAP data directory.
$project = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$data = Join-Path $project 'target\native-qa-data'
$fixtures = Join-Path $project 'target\codec-fixtures'
New-Item -ItemType Directory -Force $data | Out-Null
& cargo run -p rap-app --bin rap-index --offline -- $data $fixtures
if ($LASTEXITCODE -ne 0) { throw 'Library smoke test failed' }
@{language=$Language;dark=(-not $Light);album_grid=(-not $List);volume=0.0} | ConvertTo-Json | Set-Content -Encoding utf8NoBOM (Join-Path $data 'settings.json')
$previousData = $env:RAP_DATA_DIR
$previousLog = $env:RUST_LOG
$env:RAP_DATA_DIR = $data
$env:RUST_LOG = 'info'
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
using System.Text;
public class RapNativeQa {
 public delegate bool Callback(IntPtr handle, IntPtr data);
 [DllImport("user32.dll")] public static extern bool EnumWindows(Callback callback, IntPtr data);
 [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint process);
 [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr h, StringBuilder text, int size);
 [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int command);
 [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out Rect rect);
 [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int w, int height, uint flags);
 [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint message, IntPtr w, IntPtr l);
 [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint flags);
 [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
 public struct Rect { public int Left, Top, Right, Bottom; }
 public static Rect PhysicalRect(IntPtr handle) {
  var previous=SetThreadDpiAwarenessContext(new IntPtr(-4));
  try { Rect rect; GetWindowRect(handle,out rect); return rect; }
  finally { SetThreadDpiAwarenessContext(previous); }
 }
 public static IntPtr Find(uint process) {
  IntPtr found = IntPtr.Zero;
  EnumWindows((h,p) => {
   uint id; GetWindowThreadProcessId(h, out id);
   var text = new StringBuilder(256); GetWindowText(h, text, 256);
   if (id == process && text.ToString() == "RAP") { found=h; return false; }
   return true;
  }, IntPtr.Zero);
  return found;
 }
}
'@
$process = $null
$handle = [IntPtr]::Zero
try {
    $process = Start-Process -FilePath (Join-Path $project 'target\debug\rap.exe') -PassThru -WindowStyle Hidden -RedirectStandardOutput (Join-Path $project 'target\native-qa-out.log') -RedirectStandardError (Join-Path $project 'target\native-qa-error.log')
    for ($attempt=0; $attempt -lt 30; $attempt++) {
        Start-Sleep -Milliseconds 200
        $process.Refresh()
        if ($process.HasExited) { throw 'RAP exited before opening its window' }
        $handle = [RapNativeQa]::Find($process.Id)
        if ($handle -ne [IntPtr]::Zero) { break }
    }
    if ($handle -eq [IntPtr]::Zero) { throw 'RAP window missing' }
    if ($CloseScenario -ne 'Early') {
    [RapNativeQa]::ShowWindow($handle,9) | Out-Null
    # Raise only this QA window while capturing, then restore normal ordering.
    [RapNativeQa]::SetWindowPos($handle,[IntPtr](-1),0,0,0,0,0x0013) | Out-Null
    Start-Sleep -Seconds 2
    function Save-RapFrame([string]$name) {
        $rect = [RapNativeQa]::PhysicalRect($handle)
        $bitmap = [Drawing.Bitmap]::new($rect.Right-$rect.Left,$rect.Bottom-$rect.Top)
        $graphics = [Drawing.Graphics]::FromImage($bitmap)
        try {
            $dc = $graphics.GetHdc()
            try { $captured = [RapNativeQa]::PrintWindow($handle,$dc,2) } finally { $graphics.ReleaseHdc($dc) }
            if (-not $captured) { throw 'Window capture failed' }
            $bitmap.Save((Join-Path $project ('target\' + $name + '.png')))
        } finally { $graphics.Dispose(); $bitmap.Dispose() }
    }
    Save-RapFrame 'qa-albums'
    [RapNativeQa]::PostMessage($handle,0x0100,[IntPtr]13,[IntPtr]0) | Out-Null
    [RapNativeQa]::PostMessage($handle,0x0101,[IntPtr]13,[IntPtr]0) | Out-Null
    Start-Sleep -Milliseconds 600
    Save-RapFrame 'qa-album-detail'
    [RapNativeQa]::PostMessage($handle,0x0100,[IntPtr]32,[IntPtr]0) | Out-Null
    [RapNativeQa]::PostMessage($handle,0x0101,[IntPtr]32,[IntPtr]0) | Out-Null
    Start-Sleep -Milliseconds 500
    Save-RapFrame 'qa-playing'
    [RapNativeQa]::SetWindowPos($handle,[IntPtr](-2),0,0,0,0,0x0013) | Out-Null
    }
    if ($CloseScenario -eq 'Minimized') {
        [RapNativeQa]::ShowWindow($handle,6) | Out-Null
        Start-Sleep -Milliseconds 200
    }
    [RapNativeQa]::PostMessage($handle,0x0010,[IntPtr]0,[IntPtr]0) | Out-Null
    if (-not $process.WaitForExit(5000)) { throw 'RAP did not close cleanly' }
    if ($process.ExitCode -ne 0) { throw ('RAP exited with ' + $process.ExitCode) }
    Get-Content (Join-Path $project 'target\native-qa-error.log')
    $unexpected = Select-String -Path (Join-Path $project 'target\native-qa-out.log'),(Join-Path $project 'target\native-qa-error.log') -Pattern 'ERROR|panicked at|window not found|Handle de fenêtre non valide'
    if ($unexpected) { throw ('Unexpected RAP diagnostics: ' + ($unexpected.Line -join '; ')) }
    Write-Output ('Native RAP startup, library and close: OK (' + $CloseScenario + ')')
} finally {
    if ($process -and -not $process.HasExited) {
        [RapNativeQa]::SetWindowPos($handle,[IntPtr](-2),0,0,0,0,0x0013) | Out-Null
        [RapNativeQa]::PostMessage($handle,0x0010,[IntPtr]0,[IntPtr]0) | Out-Null
        if (-not $process.WaitForExit(3000)) { $process.Kill() }
    }
    $env:RAP_DATA_DIR=$previousData
    $env:RUST_LOG=$previousLog
}
