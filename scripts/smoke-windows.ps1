[CmdletBinding()]
param(
    [ValidateSet('French','English')][string]$Language='French',
    [switch]$Light,
    [switch]$List,
    [switch]$Taskbar,
    [switch]$AudioOutputs,
    [switch]$NetworkOutputs,
    [switch]$Collections,
    [switch]$Updates,
    [int]$WindowWidth=0,
    [int]$WindowHeight=0,
    [ValidateRange(1,16)][int]$LocalOutputCount=1,
    [ValidateRange(0,16)][int]$AudioOutputRow=0,
    [string]$Executable='Soliton.exe',
    [ValidateSet('Playing','Early','Minimized','Hidden','Ask')][string]$CloseScenario='Playing'
)
$ErrorActionPreference = 'Stop'
# Native QA uses only generated fixtures and an isolated Soliton data directory.
$project = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$data = Join-Path $project 'target\native-qa-data'
$fixtures = Join-Path $project 'target\codec-fixtures'
New-Item -ItemType Directory -Force $data | Out-Null
& cargo run -p soliton-app --bin Soliton-index --offline -- $data $fixtures
if ($LASTEXITCODE -ne 0) { throw 'Library smoke test failed' }
$closeBehavior = switch ($CloseScenario) { 'Hidden' { 'Hide' } 'Ask' { 'Ask' } default { 'Quit' } }
$initialSettings = @{language=$Language;dark=(-not $Light);album_grid=(-not $List);volume=0.0;close_behavior=$closeBehavior;check_updates=$false}
if ($Updates) { $initialSettings.last_view = @{view='Settings'}; $initialSettings.check_updates = $false }
$initialSettings | ConvertTo-Json | Set-Content -Encoding utf8NoBOM (Join-Path $data 'settings.json')
$previousData = $env:SOLITON_DATA_DIR
$previousLog = $env:RUST_LOG
$env:SOLITON_DATA_DIR = $data
$env:RUST_LOG = 'info'
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
using System.Text;
public class SolitonNativeQa {
 public delegate bool Callback(IntPtr handle, IntPtr data);
 [DllImport("user32.dll")] public static extern bool EnumWindows(Callback callback, IntPtr data);
 [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint process);
 [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr h, StringBuilder text, int size);
 [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
 [DllImport("user32.dll")] public static extern int GetWindowLongW(IntPtr h, int index);
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
  return FindTitle(process, "Soliton");
 }
 public static IntPtr FindTitle(uint process, string title) {
  IntPtr found = IntPtr.Zero;
  EnumWindows((h,p) => {
   uint id; GetWindowThreadProcessId(h, out id);
   var text = new StringBuilder(256); GetWindowText(h, text, 256);
   if (id == process && text.ToString() == title && (title != "Soliton" || (GetWindowLongW(h,-20) & 0x80) == 0)) { found=h; return false; }
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
        if ($process.HasExited) { throw 'Soliton exited before opening its window' }
        $handle = [SolitonNativeQa]::Find($process.Id)
        if ($handle -ne [IntPtr]::Zero) { break }
    }
    if ($handle -eq [IntPtr]::Zero) { throw 'Soliton window missing' }
    if ($CloseScenario -ne 'Early') {
    [SolitonNativeQa]::ShowWindow($handle,9) | Out-Null
    # Raise only this QA window while capturing, then restore normal ordering.
    [SolitonNativeQa]::SetWindowPos($handle,[IntPtr](-1),0,0,0,0,0x0013) | Out-Null
    if ($WindowWidth -gt 0 -and $WindowHeight -gt 0) {
        $previousDpi = [SolitonNativeQa]::SetThreadDpiAwarenessContext([IntPtr](-4))
        try { [SolitonNativeQa]::SetWindowPos($handle,[IntPtr]0,0,0,$WindowWidth,$WindowHeight,0x0016) | Out-Null }
        finally { [SolitonNativeQa]::SetThreadDpiAwarenessContext($previousDpi) | Out-Null }
    }
    Start-Sleep -Seconds 2
    function Save-SolitonFrame([string]$name) {
        $rect = [SolitonNativeQa]::PhysicalRect($handle)
        $bitmap = [Drawing.Bitmap]::new($rect.Right-$rect.Left,$rect.Bottom-$rect.Top)
        $graphics = [Drawing.Graphics]::FromImage($bitmap)
        try {
            $dc = $graphics.GetHdc()
            try { $captured = [SolitonNativeQa]::PrintWindow($handle,$dc,2) } finally { $graphics.ReleaseHdc($dc) }
            if (-not $captured) { throw 'Window capture failed' }
            $bitmap.Save((Join-Path $project ('target\' + $name + '.png')))
        } finally { $graphics.Dispose(); $bitmap.Dispose() }
    }
    Save-SolitonFrame 'qa-albums'
    if ($Updates) {
        $rect = [SolitonNativeQa]::PhysicalRect($handle)
        $scale = [SolitonNativeQa]::GetDpiForWindow($handle) / 96.0
        Save-SolitonFrame 'qa-update-settings-top'
        # Position over the settings content, then scroll to its final section.
        $scrollX = [int](($rect.Right-$rect.Left) / 2)
        $scrollY = [int](($rect.Bottom-$rect.Top) / 3)
        $screenPoint = ([int]($rect.Top + $scrollY) -shl 16) -bor [int]($rect.Left + $scrollX)
        $previousDpi = [SolitonNativeQa]::SetThreadDpiAwarenessContext([IntPtr](-4))
        try {
            $clientPoint = ($scrollY -shl 16) -bor $scrollX
            [SolitonNativeQa]::PostMessage($handle,0x0200,[IntPtr]0,[IntPtr]$clientPoint) | Out-Null
            for ($i=0; $i -lt 30; $i++) {
                [SolitonNativeQa]::PostMessage($handle,0x020A,[IntPtr](-7864320),[IntPtr]$screenPoint) | Out-Null
            }
        } finally { [SolitonNativeQa]::SetThreadDpiAwarenessContext($previousDpi) | Out-Null }
        Start-Sleep -Milliseconds 600
        Save-SolitonFrame 'qa-update-settings-bottom'
    }
    [SolitonNativeQa]::PostMessage($handle,0x0100,[IntPtr]13,[IntPtr]0) | Out-Null
    [SolitonNativeQa]::PostMessage($handle,0x0101,[IntPtr]13,[IntPtr]0) | Out-Null
    Start-Sleep -Milliseconds 600
    Save-SolitonFrame 'qa-album-detail'
    [SolitonNativeQa]::PostMessage($handle,0x0100,[IntPtr]32,[IntPtr]0) | Out-Null
    [SolitonNativeQa]::PostMessage($handle,0x0101,[IntPtr]32,[IntPtr]0) | Out-Null
    Start-Sleep -Milliseconds 500
    Save-SolitonFrame 'qa-playing'
    if ($Collections) {
        $scale = [SolitonNativeQa]::GetDpiForWindow($handle) / 96.0
        function Click-SolitonCollection([double]$x, [double]$y) {
            $previousDpi = [SolitonNativeQa]::SetThreadDpiAwarenessContext([IntPtr](-4))
            try {
            $point = ([int]($y*$scale) -shl 16) -bor [int]($x*$scale)
            [SolitonNativeQa]::PostMessage($handle,0x0200,[IntPtr]0,[IntPtr]$point) | Out-Null
            Start-Sleep -Milliseconds 80
            [SolitonNativeQa]::PostMessage($handle,0x0201,[IntPtr]1,[IntPtr]$point) | Out-Null
            Start-Sleep -Milliseconds 80
            [SolitonNativeQa]::PostMessage($handle,0x0202,[IntPtr]0,[IntPtr]$point) | Out-Null
            } finally { [SolitonNativeQa]::SetThreadDpiAwarenessContext($previousDpi) | Out-Null }
            Start-Sleep -Milliseconds 500
        }
        Click-SolitonCollection 385 60
        Save-SolitonFrame 'qa-queue'
        Click-SolitonCollection 105 112
        Save-SolitonFrame 'qa-playlist-name'
        [SolitonNativeQa]::PostMessage($handle,0x0100,[IntPtr]13,[IntPtr]0) | Out-Null
        [SolitonNativeQa]::PostMessage($handle,0x0101,[IntPtr]13,[IntPtr]0) | Out-Null
        Start-Sleep -Milliseconds 700
        Click-SolitonCollection 265 60
        Save-SolitonFrame 'qa-playlists'
        Click-SolitonCollection 1122 176
        Save-SolitonFrame 'qa-playlist-detail'
        $savedView = (Get-Content -Raw (Join-Path $data 'settings.json') | ConvertFrom-Json).last_view
        if ($savedView.view -ne 'Playlists' -or $null -eq $savedView.playlist) { throw 'Saved queue playlist was not opened' }
        Click-SolitonCollection 36 244
        Start-Sleep -Milliseconds 200
        [SolitonNativeQa]::PostMessage($handle,0x0100,[IntPtr]32,[IntPtr]0) | Out-Null
        [SolitonNativeQa]::PostMessage($handle,0x0101,[IntPtr]32,[IntPtr]0) | Out-Null
        Start-Sleep -Milliseconds 300
        Save-SolitonFrame 'qa-playlist-playing'
        $savedQueue = (Get-Content -Raw (Join-Path $data 'settings.json') | ConvertFrom-Json).last_queue
        if ($savedQueue.playlist_entries.Count -eq 0 -or $null -eq $savedQueue.playlist_entries[$savedQueue.current_index]) { throw 'Playing playlist occurrence was not saved' }
        # Repeatedly appending the same playlist keeps the queue and paused position.
        Click-SolitonCollection 1096 172
        Click-SolitonCollection 1096 172
        $deduplicatedQueue = (Get-Content -Raw (Join-Path $data 'settings.json') | ConvertFrom-Json).last_queue
        if ($deduplicatedQueue.tracks.Count -ne $savedQueue.tracks.Count -or
            @($deduplicatedQueue.tracks | Sort-Object -Unique).Count -ne $savedQueue.tracks.Count -or
            $deduplicatedQueue.position_ms -ne $savedQueue.position_ms) { throw 'Appending an existing playlist duplicated tracks or reset playback' }
        Click-SolitonCollection $(if ($Language -eq 'English') { 970 } else { 924 }) 112
        foreach ($character in '-QA'.ToCharArray()) {
            [SolitonNativeQa]::PostMessage($handle,0x0102,[IntPtr][int]$character,[IntPtr]0) | Out-Null
        }
        [SolitonNativeQa]::PostMessage($handle,0x0100,[IntPtr]13,[IntPtr]0) | Out-Null
        [SolitonNativeQa]::PostMessage($handle,0x0101,[IntPtr]13,[IntPtr]0) | Out-Null
        Start-Sleep -Milliseconds 500
        Save-SolitonFrame 'qa-playlist-renamed'
        Click-SolitonCollection 385 60
        Save-SolitonFrame 'qa-queue-page'
        Click-SolitonCollection 1099 228
        Save-SolitonFrame 'qa-track-actions'
        Click-SolitonCollection 1040 273
        Save-SolitonFrame 'qa-playlist-picker'
        [SolitonNativeQa]::PostMessage($handle,0x0100,[IntPtr]27,[IntPtr]0) | Out-Null
        [SolitonNativeQa]::PostMessage($handle,0x0101,[IntPtr]27,[IntPtr]0) | Out-Null
        $previousDpi = [SolitonNativeQa]::SetThreadDpiAwarenessContext([IntPtr](-4))
        try { [SolitonNativeQa]::SetWindowPos($handle,[IntPtr]0,0,0,[int](640*$scale),[int](600*$scale),0x0006) | Out-Null }
        finally { [SolitonNativeQa]::SetThreadDpiAwarenessContext($previousDpi) | Out-Null }
        Start-Sleep -Milliseconds 500
        Save-SolitonFrame 'qa-queue-compact'
    }
    if ($AudioOutputs -or $NetworkOutputs) {
        # Targeted client clicks; the user's pointer is never moved.
        $scale = [SolitonNativeQa]::GetDpiForWindow($handle) / 96.0
        function Click-SolitonClient([double]$x, [double]$y) {
            $previousDpi = [SolitonNativeQa]::SetThreadDpiAwarenessContext([IntPtr](-4))
            try {
                $point = ([int]($y*$scale) -shl 16) -bor [int]($x*$scale)
                if (-not [SolitonNativeQa]::PostMessage($handle,0x0200,[IntPtr]0,[IntPtr]$point)) { throw 'Mouse move message failed' }
                Start-Sleep -Milliseconds 100
                if (-not [SolitonNativeQa]::PostMessage($handle,0x0201,[IntPtr]1,[IntPtr]$point)) { throw 'Mouse down message failed' }
                Start-Sleep -Milliseconds 100
                if (-not [SolitonNativeQa]::PostMessage($handle,0x0202,[IntPtr]0,[IntPtr]$point)) { throw 'Mouse up message failed' }
            } finally { [SolitonNativeQa]::SetThreadDpiAwarenessContext($previousDpi) | Out-Null }
        }
        function Wait-SolitonOutputPreference([bool]$system) {
            for ($attempt=0; $attempt -lt 30; $attempt++) {
                $saved = Get-Content -Raw (Join-Path $data 'settings.json') | ConvertFrom-Json
                if (($null -eq $saved.output_device) -eq $system) { return }
                Start-Sleep -Milliseconds 100
            }
            Save-SolitonFrame 'qa-audio-rejected'
            throw 'Output preference was not acknowledged and saved'
        }
        # Keep the active stream stable while checking its selection UI.
        [SolitonNativeQa]::PostMessage($handle,0x0100,[IntPtr]32,[IntPtr]0) | Out-Null
        [SolitonNativeQa]::PostMessage($handle,0x0101,[IntPtr]32,[IntPtr]0) | Out-Null
        Start-Sleep -Milliseconds 200
        Click-SolitonClient 90 310
        Start-Sleep -Milliseconds 600
        Save-SolitonFrame 'qa-audio-outputs'
        Click-SolitonClient 400 (334 + 46.4*$AudioOutputRow)
        Wait-SolitonOutputPreference $false
        Save-SolitonFrame 'qa-audio-selected'
        Click-SolitonClient 400 286
        Wait-SolitonOutputPreference $true
        Click-SolitonClient 450 240
        Start-Sleep -Milliseconds 300
        Save-SolitonFrame 'qa-audio-default'
        if ($NetworkOutputs) {
            Click-SolitonClient 450 (394 + 46.4*($LocalOutputCount-1))
            Start-Sleep -Seconds 4
            Save-SolitonFrame 'qa-network-outputs'
        }
        Click-SolitonClient 1150 738
        Start-Sleep -Milliseconds 300
        Save-SolitonFrame 'qa-audio-now-playing'
    }
    if ($Taskbar) {
        # THBN_CLICKED | button ID: only the isolated Soliton window receives it.
        [SolitonNativeQa]::PostMessage($handle,0x0111,[IntPtr]0x18005241,[IntPtr]0) | Out-Null
        Start-Sleep -Milliseconds 300
        Save-SolitonFrame 'qa-taskbar-pause'
        foreach ($command in @(0x18005241,0x18005242,0x18005240,0x18005243)) {
            [SolitonNativeQa]::PostMessage($handle,0x0111,[IntPtr]$command,[IntPtr]0) | Out-Null
            Start-Sleep -Milliseconds 200
        }
        Save-SolitonFrame 'qa-taskbar-stop'
    }
    [SolitonNativeQa]::SetWindowPos($handle,[IntPtr](-2),0,0,0,0,0x0013) | Out-Null
    }
    if ($CloseScenario -eq 'Minimized') {
        [SolitonNativeQa]::ShowWindow($handle,6) | Out-Null
        Start-Sleep -Milliseconds 200
    }
    [SolitonNativeQa]::PostMessage($handle,0x0010,[IntPtr]0,[IntPtr]0) | Out-Null
    if ($CloseScenario -in @('Hidden','Ask')) {
        Start-Sleep -Milliseconds 600
        $process.Refresh()
        if ($process.HasExited) { throw 'Soliton unexpectedly quit on close' }
        $visible = [SolitonNativeQa]::IsWindowVisible($handle)
        if ($CloseScenario -eq 'Hidden' -and $visible) { throw 'Close did not hide Soliton' }
        if ($CloseScenario -eq 'Ask' -and -not $visible) { throw 'Close confirmation hid Soliton' }
        if ($CloseScenario -eq 'Ask') {
            Save-SolitonFrame 'qa-close-confirmation'
            [SolitonNativeQa]::PostMessage($handle,0x0100,[IntPtr]27,[IntPtr]0) | Out-Null
            [SolitonNativeQa]::PostMessage($handle,0x0101,[IntPtr]27,[IntPtr]0) | Out-Null
        }
        $owner = [SolitonNativeQa]::FindTitle($process.Id,'Soliton notification owner')
        if ($owner -eq [IntPtr]::Zero) { throw 'Soliton tray owner missing' }
        # Shell version 4 left-click selection, then its standard Quit menu command.
        [SolitonNativeQa]::PostMessage($owner,0x8006,[IntPtr]0,[IntPtr]0x0400) | Out-Null
        Start-Sleep -Milliseconds 600
        if (-not [SolitonNativeQa]::IsWindowVisible($handle)) { throw 'Tray did not restore Soliton' }
        [SolitonNativeQa]::PostMessage($owner,0x0111,[IntPtr]5,[IntPtr]0) | Out-Null
    }
    if (-not $process.WaitForExit(5000)) { throw 'Soliton did not close cleanly' }
    if ($process.ExitCode -ne 0) { throw ('Soliton exited with ' + $process.ExitCode) }
    Get-Content (Join-Path $project 'target\native-qa-error.log')
    $unexpected = Select-String -Path (Join-Path $project 'target\native-qa-out.log'),(Join-Path $project 'target\native-qa-error.log') -Pattern 'ERROR|panicked at|window not found|Handle de fenêtre non valide'
    if ($unexpected) { throw ('Unexpected Soliton diagnostics: ' + ($unexpected.Line -join '; ')) }
    Write-Output ('Native Soliton startup, library and close: OK (' + $CloseScenario + ')')
} finally {
    if ($process -and -not $process.HasExited) {
        [SolitonNativeQa]::SetWindowPos($handle,[IntPtr](-2),0,0,0,0,0x0013) | Out-Null
        [SolitonNativeQa]::PostMessage($handle,0x0010,[IntPtr]0,[IntPtr]0) | Out-Null
        $owner = [SolitonNativeQa]::FindTitle($process.Id,'Soliton notification owner')
        if ($owner -ne [IntPtr]::Zero) { [SolitonNativeQa]::PostMessage($owner,0x0111,[IntPtr]5,[IntPtr]0) | Out-Null }
        if (-not $process.WaitForExit(3000)) { $process.Kill() }
    }
    $env:SOLITON_DATA_DIR=$previousData
    $env:RUST_LOG=$previousLog
}
