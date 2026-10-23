[CmdletBinding()]
param(
    [ValidateSet('French','English')][string]$Language='French',
    [switch]$Light,
    [switch]$List,
    [switch]$Taskbar,
    [switch]$AudioOutputs,
    [switch]$NetworkOutputs,
    [switch]$Collections,
    [ValidateRange(1,16)][int]$LocalOutputCount=1,
    [ValidateRange(0,16)][int]$AudioOutputRow=0,
    [string]$Executable='rap.exe',
    [ValidateSet('Playing','Early','Minimized','Hidden','Ask')][string]$CloseScenario='Playing'
)
$ErrorActionPreference = 'Stop'
# Native QA uses only generated fixtures and an isolated RAP data directory.
$project = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$data = Join-Path $project 'target\native-qa-data'
$fixtures = Join-Path $project 'target\codec-fixtures'
New-Item -ItemType Directory -Force $data | Out-Null
& cargo run -p rap-app --bin rap-index --offline -- $data $fixtures
if ($LASTEXITCODE -ne 0) { throw 'Library smoke test failed' }
$closeBehavior = switch ($CloseScenario) { 'Hidden' { 'Hide' } 'Ask' { 'Ask' } default { 'Quit' } }
@{language=$Language;dark=(-not $Light);album_grid=(-not $List);volume=0.0;close_behavior=$closeBehavior} | ConvertTo-Json | Set-Content -Encoding utf8NoBOM (Join-Path $data 'settings.json')
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
 [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
 [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int command);
 [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out Rect rect);
 [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int w, int height, uint flags);
 [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint message, IntPtr w, IntPtr l);
 [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr h);
 [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint flags);
 [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
 public struct Rect { public int Left, Top, Right, Bottom; }
 public static Rect PhysicalRect(IntPtr handle) {
  var previous=SetThreadDpiAwarenessContext(new IntPtr(-4));
  try { Rect rect; GetWindowRect(handle,out rect); return rect; }
  finally { SetThreadDpiAwarenessContext(previous); }
 }
 public static IntPtr Find(uint process) {
  return FindTitle(process, "RAP");
 }
 public static IntPtr FindTitle(uint process, string title) {
  IntPtr found = IntPtr.Zero;
  EnumWindows((h,p) => {
   uint id; GetWindowThreadProcessId(h, out id);
   var text = new StringBuilder(256); GetWindowText(h, text, 256);
   if (id == process && text.ToString() == title) { found=h; return false; }
   return true;
  }, IntPtr.Zero);
  return found;
 }
}
'@
$process = $null
$handle = [IntPtr]::Zero
try {
    $process = Start-Process -FilePath (Join-Path $project ('target\debug\' + $Executable)) -PassThru -WindowStyle Hidden -RedirectStandardOutput (Join-Path $project 'target\native-qa-out.log') -RedirectStandardError (Join-Path $project 'target\native-qa-error.log')
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
    if ($Collections) {
        $scale = [RapNativeQa]::GetDpiForWindow($handle) / 96.0
        function Click-RapCollection([double]$x, [double]$y) {
            $previousDpi = [RapNativeQa]::SetThreadDpiAwarenessContext([IntPtr](-4))
            try {
            $point = ([int]($y*$scale) -shl 16) -bor [int]($x*$scale)
            [RapNativeQa]::PostMessage($handle,0x0200,[IntPtr]0,[IntPtr]$point) | Out-Null
            Start-Sleep -Milliseconds 80
            [RapNativeQa]::PostMessage($handle,0x0201,[IntPtr]1,[IntPtr]$point) | Out-Null
            Start-Sleep -Milliseconds 80
            [RapNativeQa]::PostMessage($handle,0x0202,[IntPtr]0,[IntPtr]$point) | Out-Null
            } finally { [RapNativeQa]::SetThreadDpiAwarenessContext($previousDpi) | Out-Null }
            Start-Sleep -Milliseconds 500
        }
        Click-RapCollection 385 60
        Save-RapFrame 'qa-queue'
        Click-RapCollection 930 136
        Save-RapFrame 'qa-playlist-name'
        [RapNativeQa]::PostMessage($handle,0x0100,[IntPtr]13,[IntPtr]0) | Out-Null
        [RapNativeQa]::PostMessage($handle,0x0101,[IntPtr]13,[IntPtr]0) | Out-Null
        Start-Sleep -Milliseconds 700
        Click-RapCollection 265 60
        Save-RapFrame 'qa-playlists'
        Click-RapCollection 1098 253
        Save-RapFrame 'qa-playlist-detail'
        $savedView = (Get-Content -Raw (Join-Path $data 'settings.json') | ConvertFrom-Json).last_view
        if ($savedView.view -ne 'Playlists' -or $null -eq $savedView.playlist) { throw 'Saved queue playlist was not opened' }
        Click-RapCollection 60 292
        Start-Sleep -Milliseconds 200
        [RapNativeQa]::PostMessage($handle,0x0100,[IntPtr]32,[IntPtr]0) | Out-Null
        [RapNativeQa]::PostMessage($handle,0x0101,[IntPtr]32,[IntPtr]0) | Out-Null
        Start-Sleep -Milliseconds 300
        Save-RapFrame 'qa-playlist-playing'
        $savedQueue = (Get-Content -Raw (Join-Path $data 'settings.json') | ConvertFrom-Json).last_queue
        if ($savedQueue.playlist_entries.Count -eq 0 -or $null -eq $savedQueue.playlist_entries[$savedQueue.current_index]) { throw 'Playing playlist occurrence was not saved' }
        # Repeatedly appending the same playlist keeps the queue and paused position.
        Click-RapCollection 180 292
        Click-RapCollection 180 292
        $deduplicatedQueue = (Get-Content -Raw (Join-Path $data 'settings.json') | ConvertFrom-Json).last_queue
        if ($deduplicatedQueue.tracks.Count -ne $savedQueue.tracks.Count -or
            @($deduplicatedQueue.tracks | Sort-Object -Unique).Count -ne $savedQueue.tracks.Count -or
            $deduplicatedQueue.position_ms -ne $savedQueue.position_ms) { throw 'Appending an existing playlist duplicated tracks or reset playback' }
        Click-RapCollection $(if ($Language -eq 'English') { 970 } else { 915 }) 126
        foreach ($character in '-QA'.ToCharArray()) {
            [RapNativeQa]::PostMessage($handle,0x0102,[IntPtr][int]$character,[IntPtr]0) | Out-Null
        }
        [RapNativeQa]::PostMessage($handle,0x0100,[IntPtr]13,[IntPtr]0) | Out-Null
        [RapNativeQa]::PostMessage($handle,0x0101,[IntPtr]13,[IntPtr]0) | Out-Null
        Start-Sleep -Milliseconds 500
        Save-RapFrame 'qa-playlist-renamed'
        Click-RapCollection 385 60
        Save-RapFrame 'qa-queue-page'
        Click-RapCollection 1118 342
        Save-RapFrame 'qa-track-actions'
        Click-RapCollection 1100 388
        Save-RapFrame 'qa-playlist-picker'
        [RapNativeQa]::PostMessage($handle,0x0100,[IntPtr]27,[IntPtr]0) | Out-Null
        [RapNativeQa]::PostMessage($handle,0x0101,[IntPtr]27,[IntPtr]0) | Out-Null
        $previousDpi = [RapNativeQa]::SetThreadDpiAwarenessContext([IntPtr](-4))
        try { [RapNativeQa]::SetWindowPos($handle,[IntPtr]0,0,0,[int](640*$scale),[int](600*$scale),0x0006) | Out-Null }
        finally { [RapNativeQa]::SetThreadDpiAwarenessContext($previousDpi) | Out-Null }
        Start-Sleep -Milliseconds 500
        Save-RapFrame 'qa-queue-compact'
    }
    if ($AudioOutputs -or $NetworkOutputs) {
        # Targeted client clicks; the user's pointer is never moved.
        $scale = [RapNativeQa]::GetDpiForWindow($handle) / 96.0
        function Click-RapClient([double]$x, [double]$y) {
            $previousDpi = [RapNativeQa]::SetThreadDpiAwarenessContext([IntPtr](-4))
            try {
                $point = ([int]($y*$scale) -shl 16) -bor [int]($x*$scale)
                if (-not [RapNativeQa]::PostMessage($handle,0x0200,[IntPtr]0,[IntPtr]$point)) { throw 'Mouse move message failed' }
                Start-Sleep -Milliseconds 100
                if (-not [RapNativeQa]::PostMessage($handle,0x0201,[IntPtr]1,[IntPtr]$point)) { throw 'Mouse down message failed' }
                Start-Sleep -Milliseconds 100
                if (-not [RapNativeQa]::PostMessage($handle,0x0202,[IntPtr]0,[IntPtr]$point)) { throw 'Mouse up message failed' }
            } finally { [RapNativeQa]::SetThreadDpiAwarenessContext($previousDpi) | Out-Null }
        }
        function Wait-RapOutputPreference([bool]$system) {
            for ($attempt=0; $attempt -lt 30; $attempt++) {
                $saved = Get-Content -Raw (Join-Path $data 'settings.json') | ConvertFrom-Json
                if (($null -eq $saved.output_device) -eq $system) { return }
                Start-Sleep -Milliseconds 100
            }
            Save-RapFrame 'qa-audio-rejected'
            throw 'Output preference was not acknowledged and saved'
        }
        # Keep the active stream stable while checking its selection UI.
        [RapNativeQa]::PostMessage($handle,0x0100,[IntPtr]32,[IntPtr]0) | Out-Null
        [RapNativeQa]::PostMessage($handle,0x0101,[IntPtr]32,[IntPtr]0) | Out-Null
        Start-Sleep -Milliseconds 200
        Click-RapClient 90 310
        Start-Sleep -Milliseconds 600
        Save-RapFrame 'qa-audio-outputs'
        Click-RapClient 400 (334 + 46.4*$AudioOutputRow)
        Wait-RapOutputPreference $false
        Save-RapFrame 'qa-audio-selected'
        Click-RapClient 400 286
        Wait-RapOutputPreference $true
        Click-RapClient 450 240
        Start-Sleep -Milliseconds 300
        Save-RapFrame 'qa-audio-default'
        if ($NetworkOutputs) {
            Click-RapClient 450 (394 + 46.4*($LocalOutputCount-1))
            Start-Sleep -Seconds 4
            Save-RapFrame 'qa-network-outputs'
        }
        Click-RapClient 1150 738
        Start-Sleep -Milliseconds 300
        Save-RapFrame 'qa-audio-now-playing'
    }
    if ($Taskbar) {
        # THBN_CLICKED | button ID: only the isolated RAP window receives it.
        [RapNativeQa]::PostMessage($handle,0x0111,[IntPtr]0x18005241,[IntPtr]0) | Out-Null
        Start-Sleep -Milliseconds 300
        Save-RapFrame 'qa-taskbar-pause'
        foreach ($command in @(0x18005241,0x18005242,0x18005240,0x18005243)) {
            [RapNativeQa]::PostMessage($handle,0x0111,[IntPtr]$command,[IntPtr]0) | Out-Null
            Start-Sleep -Milliseconds 200
        }
        Save-RapFrame 'qa-taskbar-stop'
    }
    [RapNativeQa]::SetWindowPos($handle,[IntPtr](-2),0,0,0,0,0x0013) | Out-Null
    }
    if ($CloseScenario -eq 'Minimized') {
        [RapNativeQa]::ShowWindow($handle,6) | Out-Null
        Start-Sleep -Milliseconds 200
    }
    [RapNativeQa]::PostMessage($handle,0x0010,[IntPtr]0,[IntPtr]0) | Out-Null
    if ($CloseScenario -in @('Hidden','Ask')) {
        Start-Sleep -Milliseconds 600
        $process.Refresh()
        if ($process.HasExited) { throw 'RAP unexpectedly quit on close' }
        $visible = [RapNativeQa]::IsWindowVisible($handle)
        if ($CloseScenario -eq 'Hidden' -and $visible) { throw 'Close did not hide RAP' }
        if ($CloseScenario -eq 'Ask' -and -not $visible) { throw 'Close confirmation hid RAP' }
        if ($CloseScenario -eq 'Ask') {
            Save-RapFrame 'qa-close-confirmation'
            [RapNativeQa]::PostMessage($handle,0x0100,[IntPtr]27,[IntPtr]0) | Out-Null
            [RapNativeQa]::PostMessage($handle,0x0101,[IntPtr]27,[IntPtr]0) | Out-Null
        }
        $owner = [RapNativeQa]::FindTitle($process.Id,'RAP notification owner')
        if ($owner -eq [IntPtr]::Zero) { throw 'RAP tray owner missing' }
        # Shell version 4 left-click selection, then its standard Quit menu command.
        [RapNativeQa]::PostMessage($owner,0x8006,[IntPtr]0,[IntPtr]0x0400) | Out-Null
        Start-Sleep -Milliseconds 600
        if (-not [RapNativeQa]::IsWindowVisible($handle)) { throw 'Tray did not restore RAP' }
        [RapNativeQa]::PostMessage($owner,0x0111,[IntPtr]5,[IntPtr]0) | Out-Null
    }
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
        $owner = [RapNativeQa]::FindTitle($process.Id,'RAP notification owner')
        if ($owner -ne [IntPtr]::Zero) { [RapNativeQa]::PostMessage($owner,0x0111,[IntPtr]5,[IntPtr]0) | Out-Null }
        if (-not $process.WaitForExit(3000)) { $process.Kill() }
    }
    $env:RAP_DATA_DIR=$previousData
    $env:RUST_LOG=$previousLog
}
