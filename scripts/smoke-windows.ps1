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
    [ValidateRange(0,16)][int]$AudioOutputRow=0,
    [string]$Executable='Soliton.exe',
    [ValidateSet('Playing','Early','Minimized','Hidden','Ask')][string]$CloseScenario='Playing'
)
$ErrorActionPreference = 'Stop'
# Native QA uses only generated fixtures and an isolated Soliton data directory.
$project = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$data = Join-Path $project 'target\native-qa-data'
$fixtures = Join-Path $project 'target\codec-fixtures'
# A fresh library each run: sources left by earlier runs would add notices and shift the layout.
Remove-Item -LiteralPath $data -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $data | Out-Null
& cargo run -p soliton-app --bin Soliton-index --offline -- $data $fixtures
if ($LASTEXITCODE -ne 0) { throw 'Library smoke test failed' }
$closeBehavior = switch ($CloseScenario) { 'Hidden' { 'Hide' } 'Ask' { 'Ask' } default { 'Quit' } }
$initialSettings = @{language=$Language;dark=(-not $Light);album_grid=(-not $List);volume=0.0;close_behavior=$closeBehavior;check_updates=$false}
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
 [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out Rect rect);
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
 public static Rect PhysicalClient(IntPtr handle) {
  var previous=SetThreadDpiAwarenessContext(new IntPtr(-4));
  try { Rect rect; GetClientRect(handle,out rect); return rect; }
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
    # Targeted clicks in logical client pixels; the user's pointer is never moved.
    # Positions are measured at the default rem size: the section rail from the
    # top left, the settings entry from the bottom, page buttons from the right.
    $scale = [SolitonNativeQa]::GetDpiForWindow($handle) / 96.0
    $client = [SolitonNativeQa]::PhysicalClient($handle)
    $clientWidth = $client.Right / $scale
    $clientHeight = $client.Bottom / $scale
    function Click-SolitonClient([double]$x, [double]$y) {
        $previousDpi = [SolitonNativeQa]::SetThreadDpiAwarenessContext([IntPtr](-4))
        try {
            $point = ([int]($y*$scale) -shl 16) -bor [int]($x*$scale)
            if (-not [SolitonNativeQa]::PostMessage($handle,0x0200,[IntPtr]0,[IntPtr]$point)) { throw 'Mouse move message failed' }
            Start-Sleep -Milliseconds 80
            if (-not [SolitonNativeQa]::PostMessage($handle,0x0201,[IntPtr]1,[IntPtr]$point)) { throw 'Mouse down message failed' }
            Start-Sleep -Milliseconds 80
            if (-not [SolitonNativeQa]::PostMessage($handle,0x0202,[IntPtr]0,[IntPtr]$point)) { throw 'Mouse up message failed' }
        } finally { [SolitonNativeQa]::SetThreadDpiAwarenessContext($previousDpi) | Out-Null }
        Start-Sleep -Milliseconds 500
    }
    function Scroll-SolitonClient([double]$x, [double]$y, [int]$notches) {
        $previousDpi = [SolitonNativeQa]::SetThreadDpiAwarenessContext([IntPtr](-4))
        try {
            $rect = [SolitonNativeQa]::PhysicalRect($handle)
            $point = ([int]($y*$scale) -shl 16) -bor [int]($x*$scale)
            $screenPoint = ([int]($rect.Top + $y*$scale) -shl 16) -bor [int]($rect.Left + $x*$scale)
            [SolitonNativeQa]::PostMessage($handle,0x0200,[IntPtr]0,[IntPtr]$point) | Out-Null
            for ($i=0; $i -lt $notches; $i++) {
                [SolitonNativeQa]::PostMessage($handle,0x020A,[IntPtr](-7864320),[IntPtr]$screenPoint) | Out-Null
            }
        } finally { [SolitonNativeQa]::SetThreadDpiAwarenessContext($previousDpi) | Out-Null }
        Start-Sleep -Milliseconds 600
    }
    function Send-SolitonKey([int]$key) {
        [SolitonNativeQa]::PostMessage($handle,0x0100,[IntPtr]$key,[IntPtr]0) | Out-Null
        [SolitonNativeQa]::PostMessage($handle,0x0101,[IntPtr]$key,[IntPtr]0) | Out-Null
    }
    $rail = @{Playlists=156; Queue=200}
    # Settings are never restored at startup: they open from the bottom of the rail.
    function Open-SolitonSettings { Click-SolitonClient 28 ($clientHeight - 132) }
    Save-SolitonFrame 'qa-albums'
    if ($Updates) {
        Open-SolitonSettings
        Save-SolitonFrame 'qa-update-settings-top'
        Scroll-SolitonClient ($clientWidth / 2) ($clientHeight / 3) 30
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
        Click-SolitonClient 28 $rail.Queue
        Save-SolitonFrame 'qa-queue'
        # Save as playlist, first button of the queue page.
        Click-SolitonClient 140 94
        Start-Sleep -Milliseconds 300
        Save-SolitonFrame 'qa-playlist-name'
        Send-SolitonKey 13
        Start-Sleep -Milliseconds 700
        Click-SolitonClient 28 $rail.Playlists
        Save-SolitonFrame 'qa-playlists'
        # Open, last button of the first playlist row.
        Click-SolitonClient ($clientWidth - 59) 171
        Save-SolitonFrame 'qa-playlist-detail'
        $savedView = (Get-Content -Raw (Join-Path $data 'settings.json') | ConvertFrom-Json).last_view
        if ($savedView.view -ne 'Playlists' -or $null -eq $savedView.playlist) { throw 'Saved queue playlist was not opened' }
        # Play the first track, then pause it.
        Click-SolitonClient 92 204
        Start-Sleep -Milliseconds 200
        Send-SolitonKey 32
        Start-Sleep -Milliseconds 300
        Save-SolitonFrame 'qa-playlist-playing'
        $savedQueue = (Get-Content -Raw (Join-Path $data 'settings.json') | ConvertFrom-Json).last_queue
        if ($savedQueue.playlist_entries.Count -eq 0 -or $null -eq $savedQueue.playlist_entries[$savedQueue.current_index]) { throw 'Playing playlist occurrence was not saved' }
        # Repeatedly appending the same playlist keeps the queue and paused position.
        Click-SolitonClient 330 60
        Click-SolitonClient 330 60
        $deduplicatedQueue = (Get-Content -Raw (Join-Path $data 'settings.json') | ConvertFrom-Json).last_queue
        if ($deduplicatedQueue.tracks.Count -ne $savedQueue.tracks.Count -or
            @($deduplicatedQueue.tracks | Sort-Object -Unique).Count -ne $savedQueue.tracks.Count -or
            $deduplicatedQueue.position_ms -ne $savedQueue.position_ms) { throw 'Appending an existing playlist duplicated tracks or reset playback' }
        # Rename, left of the wider Delete playlist button.
        Click-SolitonClient ($clientWidth - $(if ($Language -eq 'English') { 209 } else { 256 })) 60
        foreach ($character in '-QA'.ToCharArray()) {
            [SolitonNativeQa]::PostMessage($handle,0x0102,[IntPtr][int]$character,[IntPtr]0) | Out-Null
        }
        Send-SolitonKey 13
        Start-Sleep -Milliseconds 500
        Save-SolitonFrame 'qa-playlist-renamed'
        Click-SolitonClient 28 $rail.Queue
        Save-SolitonFrame 'qa-queue-page'
        # Actions of the first queued track, then Add to playlist.
        Click-SolitonClient ($clientWidth - 81) 223
        Save-SolitonFrame 'qa-track-actions'
        Click-SolitonClient ($clientWidth - 120) 268
        Save-SolitonFrame 'qa-playlist-picker'
        Send-SolitonKey 27
        $previousDpi = [SolitonNativeQa]::SetThreadDpiAwarenessContext([IntPtr](-4))
        try { [SolitonNativeQa]::SetWindowPos($handle,[IntPtr]0,0,0,[int](640*$scale),[int](600*$scale),0x0006) | Out-Null }
        finally { [SolitonNativeQa]::SetThreadDpiAwarenessContext($previousDpi) | Out-Null }
        Start-Sleep -Milliseconds 500
        Save-SolitonFrame 'qa-queue-compact'
    }
    if ($AudioOutputs -or $NetworkOutputs) {
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
        Send-SolitonKey 32
        Start-Sleep -Milliseconds 200
        # Four notches bring the Audio output list into view: default system
        # device first, then one row per device.
        Open-SolitonSettings
        Scroll-SolitonClient 500 400 4
        Save-SolitonFrame 'qa-audio-outputs'
        Click-SolitonClient 300 (487 + 41*$AudioOutputRow)
        Wait-SolitonOutputPreference $false
        Save-SolitonFrame 'qa-audio-selected'
        Click-SolitonClient 300 446
        Wait-SolitonOutputPreference $true
        Save-SolitonFrame 'qa-audio-default'
        if ($NetworkOutputs) {
            # Find devices, measured from the bottom of the scrolled-down page:
            # paired Bluetooth devices above it vary between machines.
            Scroll-SolitonClient 500 400 30
            Click-SolitonClient 811 ($clientHeight - $(if ($Language -eq 'English') { 686 } else { 705 }))
            Start-Sleep -Seconds 4
            Save-SolitonFrame 'qa-network-outputs'
        }
        # Listening mode, last button of the player bar.
        Click-SolitonClient ($clientWidth - 28) ($clientHeight - 40)
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
