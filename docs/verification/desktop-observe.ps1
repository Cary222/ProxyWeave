param([int]$ProcessId, [string]$Action = 'observe', [string]$Name = '', [long]$WindowHandle = 0)
[Console]::OutputEncoding = [Text.Encoding]::UTF8
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class ShellProbe {
 [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hwnd);
 [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr hwnd, uint message, IntPtr w, IntPtr l);
}
'@
$root = [System.Windows.Automation.AutomationElement]::RootElement
$condition = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ProcessIdProperty, $ProcessId)
$windows = $root.FindAll([System.Windows.Automation.TreeScope]::Children, $condition)
if ($Action -eq 'close') {
 if ($WindowHandle -eq 0) { throw 'Exact observed handle required' }
 [ShellProbe]::PostMessage([IntPtr]$WindowHandle, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero) | Out-Null
} elseif ($Action -eq 'invoke') {
 $found = $false
 foreach ($w in $windows) {
  if ($WindowHandle -ne 0 -and $w.Current.NativeWindowHandle -ne $WindowHandle) { continue }
  $c = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::NameProperty, $Name)
  $e = $w.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $c)
  if ($e) {
   ([System.Windows.Automation.InvokePattern]$e.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)).Invoke()
   $found = $true; break
  }
 }
 if (-not $found) { throw "Control not found: $Name" }
}
$result = @()
foreach ($w in $windows) {
 $buttons = @(); $texts = @()
 foreach ($e in $w.FindAll([System.Windows.Automation.TreeScope]::Descendants,[System.Windows.Automation.Condition]::TrueCondition)) {
  $kind = $e.Current.ControlType.ProgrammaticName
  if ($kind -eq 'ControlType.Button') { $buttons += $e.Current.Name }
  if ($kind -eq 'ControlType.Text') { $texts += $e.Current.Name }
 }
 $result += @{title=$w.Current.Name; handle=$w.Current.NativeWindowHandle; visible=[ShellProbe]::IsWindowVisible([IntPtr]$w.Current.NativeWindowHandle); buttons=$buttons; texts=$texts}
}
@{processId=$ProcessId; action=$Action; windows=$result; alive=[bool](Get-Process -Id $ProcessId -ErrorAction SilentlyContinue)} | ConvertTo-Json -Depth 6 -Compress
