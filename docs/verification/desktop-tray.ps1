param([string]$Title = 'ProxyWeave', [string]$Action = 'observe')
[Console]::OutputEncoding = [Text.Encoding]::UTF8
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
$root = [System.Windows.Automation.AutomationElement]::RootElement
# Windows 11 places notification icons in a separate overflow XAML window.
if ($Action -eq 'open' -or $Action -eq 'invoke') {
 $overflow = $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, (New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::NameProperty, '显示隐藏的图标')))
 if ($overflow) {
  ([System.Windows.Automation.InvokePattern]$overflow.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)).Invoke()
 }
}
$items = @()
foreach ($w in $root.FindAll([System.Windows.Automation.TreeScope]::Children,[System.Windows.Automation.Condition]::TrueCondition)) {
 if ($w.Current.ClassName -notmatch 'Tray|Overflow') { continue }
 foreach ($e in $w.FindAll([System.Windows.Automation.TreeScope]::Descendants,[System.Windows.Automation.Condition]::TrueCondition)) {
  if ($e.Current.Name -notlike "*$Title*") { continue }
  $items += @{name=$e.Current.Name; bounds=$e.Current.BoundingRectangle.ToString(); patterns=@($e.GetSupportedPatterns() | ForEach-Object { $_.ProgrammaticName })}
  if ($Action -eq 'invoke') {
   ([System.Windows.Automation.InvokePattern]$e.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)).Invoke()
  }
 }
}
@{action=$Action; items=$items} | ConvertTo-Json -Depth 4 -Compress
