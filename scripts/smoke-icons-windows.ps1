[CmdletBinding()]
param(
    [ValidateSet('French','English')][string]$Language='French',
    [switch]$Light
)
$ErrorActionPreference='Stop'
# All settings and captures belong to this isolated, reproducible QA profile.
$project=(Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$data=Join-Path $project 'target\icon-qa-data'
$output=Join-Path $project ('target\icon-qa-' + $Language + $(if($Light){'-light'}else{'-dark'}))
New-Item -ItemType Directory -Force -Path $data,$output | Out-Null
$settings=Join-Path $data 'settings.json'
@{language=$Language;appearance=$(if($Light){'Light'}else{'Dark'});volume=0;close_behavior='Hide';check_updates=$false;window=@{x=100;y=100;width=1180;height=880;maximized=$false}} | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $settings -Encoding utf8NoBOM
$previousData=$env:SOLITON_DATA_DIR
$previousLog=$env:RUST_LOG
$env:SOLITON_DATA_DIR=$data
$env:RUST_LOG='info'
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Text;
using System.Runtime.InteropServices;
public class SolitonIconsSmoke {
 public delegate bool Callback(IntPtr h,IntPtr p);
 [DllImport("user32.dll")] public static extern bool EnumWindows(Callback cb,IntPtr p);
 [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h,out uint p);
 [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr h,StringBuilder s,int n);
 [DllImport("user32.dll")] public static extern int GetWindowLongW(IntPtr h,int i);
 [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h,int c);
 [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
 [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h,uint m,IntPtr w,IntPtr l);
 [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h,out Rect r);
 [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h,ref Point p);
 [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h,out Rect r);
 [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h,IntPtr dc,uint f);
 [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr c);
 [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr h);
 [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr h,uint m,IntPtr w,IntPtr l);
 public struct Rect { public int Left,Top,Right,Bottom; }
 public struct Point { public int X,Y; }
 public static IntPtr Find(uint process,string title,int tool) {
  IntPtr found=IntPtr.Zero;
  EnumWindows((h,p)=>{ uint id;GetWindowThreadProcessId(h,out id);var s=new StringBuilder(256);GetWindowText(h,s,256);
   bool isTool=(GetWindowLongW(h,-20)&0x80)!=0;
   if(id==process && s.ToString()==title && (tool<0 || isTool==(tool==1))){found=h;return false;}return true;
  },IntPtr.Zero);return found;
 }
}
'@
$process=$null
$handle=[IntPtr]::Zero
function Start-IconTest([string]$suffix) {
    $script:process=Start-Process -FilePath (Join-Path $project 'target\debug\Soliton.exe') -PassThru -WindowStyle Hidden -RedirectStandardOutput (Join-Path $output ($suffix+'-out.log')) -RedirectStandardError (Join-Path $output ($suffix+'-error.log'))
    for($attempt=0;$attempt -lt 40;$attempt++) {
        Start-Sleep -Milliseconds 150
        $script:handle=[SolitonIconsSmoke]::Find($process.Id,'Soliton',0)
        if($handle -ne [IntPtr]::Zero){break}
    }
    if($handle -eq [IntPtr]::Zero){throw 'Main soliton window missing or title is not Soliton'}
    [SolitonIconsSmoke]::ShowWindow($handle,9) | Out-Null
    Start-Sleep -Seconds 1
}
function Save-IconFrame([string]$name) {
    $oldDpi=[SolitonIconsSmoke]::SetThreadDpiAwarenessContext([IntPtr](-4))
    try {
        $rect=New-Object SolitonIconsSmoke+Rect
        [SolitonIconsSmoke]::GetWindowRect($handle,[ref]$rect) | Out-Null
        $bitmap=[Drawing.Bitmap]::new($rect.Right-$rect.Left,$rect.Bottom-$rect.Top)
        $graphics=[Drawing.Graphics]::FromImage($bitmap)
        try {
            $dc=$graphics.GetHdc()
            try {if(-not [SolitonIconsSmoke]::PrintWindow($handle,$dc,2)){throw 'Capture failed'}} finally {$graphics.ReleaseHdc($dc)}
            $bitmap.Save((Join-Path $output ($name+'.png')))
        } finally {$graphics.Dispose();$bitmap.Dispose()}
    } finally {[SolitonIconsSmoke]::SetThreadDpiAwarenessContext($oldDpi) | Out-Null}
}
function Open-Settings {
    # Settings are never restored at startup: their rail entry sits 132 logical
    # pixels above the bottom of the client area.
    $scale=[SolitonIconsSmoke]::GetDpiForWindow($handle)/96.0
    $oldDpi=[SolitonIconsSmoke]::SetThreadDpiAwarenessContext([IntPtr](-4))
    try {
        $client=New-Object SolitonIconsSmoke+Rect
        [SolitonIconsSmoke]::GetClientRect($handle,[ref]$client) | Out-Null
        $point=([int]($client.Bottom-132*$scale) -shl 16) -bor [int](28*$scale)
        [SolitonIconsSmoke]::PostMessage($handle,0x200,[IntPtr]0,[IntPtr]$point) | Out-Null
        [SolitonIconsSmoke]::PostMessage($handle,0x201,[IntPtr]1,[IntPtr]$point) | Out-Null
        [SolitonIconsSmoke]::PostMessage($handle,0x202,[IntPtr]0,[IntPtr]$point) | Out-Null
    } finally {[SolitonIconsSmoke]::SetThreadDpiAwarenessContext($oldDpi) | Out-Null}
    Start-Sleep -Milliseconds 600
}
function Click-Icon([int]$index) {
    # Centers of the six icon cards, measured at the fixed QA window/rem size.
    $scale=[SolitonIconsSmoke]::GetDpiForWindow($handle)/96.0
    $oldDpi=[SolitonIconsSmoke]::SetThreadDpiAwarenessContext([IntPtr](-4))
    try {
        $rect=New-Object SolitonIconsSmoke+Rect
        $origin=New-Object SolitonIconsSmoke+Point
        [SolitonIconsSmoke]::GetWindowRect($handle,[ref]$rect) | Out-Null
        [SolitonIconsSmoke]::ClientToScreen($handle,[ref]$origin) | Out-Null
        $x=[int]((116+$index*64)*$scale)-($origin.X-$rect.Left)
        $y=[int](475*$scale)-($origin.Y-$rect.Top)
        $point=($y -shl 16) -bor ($x -band 0xffff)
        [SolitonIconsSmoke]::PostMessage($handle,0x200,[IntPtr]0,[IntPtr]$point) | Out-Null
        [SolitonIconsSmoke]::PostMessage($handle,0x201,[IntPtr]1,[IntPtr]$point) | Out-Null
        [SolitonIconsSmoke]::PostMessage($handle,0x202,[IntPtr]0,[IntPtr]$point) | Out-Null
    } finally {[SolitonIconsSmoke]::SetThreadDpiAwarenessContext($oldDpi) | Out-Null}
}
function Assert-IconColor([string]$color) {
    # WM_GETICON covers both the main window and its hidden taskbar thumbnail.
    $proxy=[SolitonIconsSmoke]::Find($process.Id,'Soliton',1)
    if($proxy -eq [IntPtr]::Zero){throw 'Taskbar thumbnail proxy missing'}
    foreach($window in @($handle,$proxy)) {
        foreach($kind in @(0,1)) {
            $iconHandle=[SolitonIconsSmoke]::SendMessage($window,0x7f,[IntPtr]$kind,[IntPtr]0)
            if($iconHandle -eq [IntPtr]::Zero){throw 'Native icon missing'}
            $icon=[Drawing.Icon]::FromHandle($iconHandle)
            $bitmap=$icon.ToBitmap()
            try {
                $pixel=$bitmap.GetPixel([int]($bitmap.Width/2),[math]::Max(1,[int]($bitmap.Height/8)))
                $actual='#{0:X2}{1:X2}{2:X2}' -f $pixel.R,$pixel.G,$pixel.B
                $expected=[Drawing.ColorTranslator]::FromHtml($color)
                # GDI's icon conversion can round RGB channels down by one.
                if([math]::Abs([int]$pixel.R-$expected.R) -gt 2 -or [math]::Abs([int]$pixel.G-$expected.G) -gt 2 -or [math]::Abs([int]$pixel.B-$expected.B) -gt 2){throw "Expected native icon $color, got $actual"}
            } finally {$bitmap.Dispose();$icon.Dispose()}
        }
    }
}
function Quit-IconTest {
    $owner=[SolitonIconsSmoke]::Find($process.Id,'Soliton notification owner',-1)
    if($owner -eq [IntPtr]::Zero){throw 'Tray owner missing'}
    [SolitonIconsSmoke]::PostMessage($owner,0x111,[IntPtr]5,[IntPtr]0) | Out-Null
    if(-not $process.WaitForExit(5000)){throw 'soliton did not exit cleanly'}
    if($process.ExitCode -ne 0){throw ('soliton exited with '+$process.ExitCode)}
}
try {
    Start-IconTest 'first'
    Assert-IconColor '#23272F'
    Open-Settings
    Save-IconFrame 'default-graphite-mint'
    $palettes=@('Blue','Teal','Violet','Copper','GraphiteMint','IvoryInk')
    $colors=@('#315EF4','#086E75','#7544D8','#B65A32','#23272F','#EEE6D7')
    for($index=0;$index -lt $palettes.Count;$index++) {
        Click-Icon $index
        $saved=$false
        for($attempt=0;$attempt -lt 30;$attempt++) {
            Start-Sleep -Milliseconds 100
            try {$saved=((Get-Content -LiteralPath $settings -Raw | ConvertFrom-Json).app_icon -eq $palettes[$index])} catch {$saved=$false}
            if($saved){break}
        }
        if(-not $saved){throw ('Icon choice was not saved: '+$palettes[$index])}
        # Native icons are rendered off the UI thread after the choice is saved.
        for($attempt=0;;$attempt++) {
            Start-Sleep -Milliseconds 250
            try {Assert-IconColor $colors[$index];break} catch {if($attempt -ge 11){throw}}
        }
        Save-IconFrame $palettes[$index]
    }
    # A successful tray update must keep close-to-tray and restoration available.
    [SolitonIconsSmoke]::PostMessage($handle,0x10,[IntPtr]0,[IntPtr]0) | Out-Null
    Start-Sleep -Milliseconds 500
    if($process.HasExited -or [SolitonIconsSmoke]::IsWindowVisible($handle)){throw 'Close-to-tray failed after changing icons'}
    $owner=[SolitonIconsSmoke]::Find($process.Id,'Soliton notification owner',-1)
    [SolitonIconsSmoke]::PostMessage($owner,0x8006,[IntPtr]0,[IntPtr]0x400) | Out-Null
    Start-Sleep -Milliseconds 500
    if(-not [SolitonIconsSmoke]::IsWindowVisible($handle)){throw 'Tray did not restore soliton'}
    Quit-IconTest
    Start-IconTest 'restored'
    Assert-IconColor '#EEE6D7'
    Save-IconFrame 'restored-ivory-ink'
    Quit-IconTest
    $errors=Select-String -Path (Join-Path $output '*-out.log'),(Join-Path $output '*-error.log') -Pattern 'ERROR|panicked at|Window icon:|Tray icon:|Taskbar icon:'
    if($errors){throw ('Unexpected icon diagnostics: '+($errors.Line -join '; '))}
    Write-Output "Six icon choices, native window/proxy icons, persistence, Soliton title and tray lifecycle: OK ($Language, Light=$Light)"
} finally {
    if($process -and -not $process.HasExited) {
        $owner=[SolitonIconsSmoke]::Find($process.Id,'Soliton notification owner',-1)
        if($owner -ne [IntPtr]::Zero){[SolitonIconsSmoke]::PostMessage($owner,0x111,[IntPtr]5,[IntPtr]0)|Out-Null}
        if(-not $process.WaitForExit(3000)){$process.Kill()}
    }
    $env:SOLITON_DATA_DIR=$previousData
    $env:RUST_LOG=$previousLog
}
