$drivers = @{}
foreach ($d in (Get-CimInstance Win32_PnPSignedDriver)) {
  if ($d.DeviceID) { $drivers[$d.DeviceID] = $d }
}
$items = New-Object System.Collections.ArrayList
foreach ($e in (Get-CimInstance Win32_PnPEntity)) {
  $d = $drivers[$e.PNPDeviceID]
  # CIM returns DriverDate as local time; the stored value is UTC midnight, so convert back.
  $date = $null
  if ($d -and $d.DriverDate) { $date = $d.DriverDate.ToUniversalTime().ToString('yyyy-MM-dd') }
  [void]$items.Add([pscustomobject]@{
    instanceId    = [string]$e.PNPDeviceID
    name          = [string]$e.Name
    class         = [string]$e.PNPClass
    manufacturer  = [string]$e.Manufacturer
    provider      = if ($d) { [string]$d.DriverProviderName } else { '' }
    version       = if ($d) { [string]$d.DriverVersion } else { '' }
    date          = $date
    inf           = if ($d) { [string]$d.InfName } else { '' }
    hardwareIds   = @($e.HardwareID | Where-Object { $_ } | ForEach-Object { [string]$_ })
    compatibleIds = @($e.CompatibleID | Where-Object { $_ } | ForEach-Object { [string]$_ })
    signed        = if ($d) { [bool]$d.IsSigned } else { $false }
    signer        = if ($d) { [string]$d.Signer } else { '' }
    problemCode   = [int]$e.ConfigManagerErrorCode
  })
}
ConvertTo-Json -InputObject @($items) -Depth 4 -Compress
