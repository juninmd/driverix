$session = New-Object -ComObject Microsoft.Update.Session
$result = $session.CreateUpdateSearcher().Search("IsInstalled=0 and Type='Driver' and IsHidden=0")
$items = New-Object System.Collections.ArrayList
foreach ($u in $result.Updates) {
  $date = $null
  if ($u.DriverVerDate) { $date = $u.DriverVerDate.ToUniversalTime().ToString('yyyy-MM-dd') }
  [void]$items.Add([pscustomobject]@{
    updateId       = [string]$u.Identity.UpdateID
    revision       = [int]$u.Identity.RevisionNumber
    title          = [string]$u.Title
    class          = [string]$u.DriverClass
    manufacturer   = [string]$u.DriverManufacturer
    provider       = [string]$u.DriverProvider
    model          = [string]$u.DriverModel
    hardwareId     = [string]$u.DriverHardwareID
    date           = $date
    sizeBytes      = [long]$u.MaxDownloadSize
    rebootRequired = ([int]$u.InstallationBehavior.RebootBehavior -ne 0)
  })
}
ConvertTo-Json -InputObject @($items) -Depth 3 -Compress
