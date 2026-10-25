function S($v) { if ($null -eq $v) { '' } else { ([string]$v).Trim() } }
function N($v) { try { if ($null -eq $v) { [uint64]0 } else { [uint64]$v } } catch { [uint64]0 } }
function Try-Get([scriptblock]$b) { try { & $b } catch { $null } }
# CIM returns dates as local time; the stored value is UTC midnight, so convert back.
function Day($d) { if ($d) { $d.ToUniversalTime().ToString('yyyy-MM-dd') } else { $null } }
function Unix($d) { if ($d) { [uint64]([DateTimeOffset]$d).ToUnixTimeSeconds() } else { [uint64]0 } }

$cs = Try-Get { Get-CimInstance Win32_ComputerSystem }
$os = Try-Get { Get-CimInstance Win32_OperatingSystem }
$bb = Try-Get { Get-CimInstance Win32_BaseBoard }
$bios = Try-Get { Get-CimInstance Win32_BIOS }
$cv = Try-Get { Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion' }

$cpus = @(Try-Get { Get-CimInstance Win32_Processor } | ForEach-Object {
  [pscustomobject]@{
    name = (S $_.Name); manufacturer = (S $_.Manufacturer); architecture = [int](N $_.Architecture)
    cores = [int](N $_.NumberOfCores); threads = [int](N $_.NumberOfLogicalProcessors)
    maxMhz = [int](N $_.MaxClockSpeed); socket = (S $_.SocketDesignation)
    l2Kb = [int](N $_.L2CacheSize); l3Kb = [int](N $_.L3CacheSize)
  }
})

$slots = 0
foreach ($a in @(Try-Get { Get-CimInstance Win32_PhysicalMemoryArray })) { $slots += [int](N $a.MemoryDevices) }
$modules = @(Try-Get { Get-CimInstance Win32_PhysicalMemory } | ForEach-Object {
  [pscustomobject]@{
    slot = (S $_.DeviceLocator); sizeBytes = (N $_.Capacity); speedMhz = [int](N $_.Speed)
    memoryType = [int](N $_.SMBIOSMemoryType); manufacturer = (S $_.Manufacturer); part = (S $_.PartNumber)
  }
})

# Win32_VideoController.AdapterRAM is a 32-bit value capped at 4 GB; the driver key has the real size.
$vram = @{}
try {
  $cls = 'HKLM:\SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}'
  foreach ($k in @(Get-ChildItem $cls -ErrorAction SilentlyContinue)) {
    try {
      $p = Get-ItemProperty $k.PSPath
      $size = $p.'HardwareInformation.qwMemorySize'
      if ($p.DriverDesc -and $size) { $vram[[string]$p.DriverDesc] = [uint64]$size }
    } catch { }
  }
} catch { }
$gpus = @(Try-Get { Get-CimInstance Win32_VideoController } | ForEach-Object {
  $mem = $vram[(S $_.Name)]
  if (-not $mem) { $mem = N $_.AdapterRAM }
  [pscustomobject]@{
    name = (S $_.Name); vendor = (S $_.AdapterCompatibility); vramBytes = [uint64]$mem
    driverVersion = (S $_.DriverVersion); driverDate = (Day $_.DriverDate)
    width = [int](N $_.CurrentHorizontalResolution); height = [int](N $_.CurrentVerticalResolution)
    refreshHz = [int](N $_.CurrentRefreshRate)
  }
})

$disks = @()
try {
  $disks = @(Get-PhysicalDisk | Sort-Object DeviceId | ForEach-Object {
    [pscustomobject]@{
      model = (S $_.FriendlyName); sizeBytes = (N $_.Size); media = (S $_.MediaType)
      bus = (S $_.BusType); health = (S $_.HealthStatus); firmware = (S $_.FirmwareVersion)
    }
  })
} catch {
  $disks = @(Try-Get { Get-CimInstance Win32_DiskDrive } | ForEach-Object {
    [pscustomobject]@{
      model = (S $_.Model); sizeBytes = (N $_.Size); media = ''
      bus = (S $_.InterfaceType); health = (S $_.Status); firmware = (S $_.FirmwareRevision)
    }
  })
}
$volumes = @(Try-Get { Get-CimInstance Win32_LogicalDisk -Filter 'DriveType=3' } | ForEach-Object {
  [pscustomobject]@{
    letter = (S $_.DeviceID); label = (S $_.VolumeName); fs = (S $_.FileSystem)
    sizeBytes = (N $_.Size); freeBytes = (N $_.FreeSpace)
  }
})

$nics = @(Try-Get { Get-CimInstance Win32_NetworkAdapter -Filter 'PhysicalAdapter=True' } | ForEach-Object {
  [pscustomobject]@{
    name = (S $_.Name); mac = (S $_.MACAddress); speedBps = (N $_.Speed)
    connected = ([int](N $_.NetConnectionStatus) -eq 2)
  }
})

$battery = $null
$b = Try-Get { Get-CimInstance Win32_Battery | Select-Object -First 1 }
if ($b) {
  $battery = [pscustomobject]@{
    name = (S $b.Name); statusCode = [int](N $b.BatteryStatus)
    # Null means the battery does not report a charge; 0 would read as an empty battery.
    chargePercent = if ($null -eq $b.EstimatedChargeRemaining) { $null } else { [int]$b.EstimatedChargeRemaining }
  }
}

$boot = if ($os) { $os.LastBootUpTime } else { $null }
$info = [pscustomobject]@{
  system = [pscustomobject]@{
    manufacturer = (S $cs.Manufacturer); model = (S $cs.Model); family = (S $cs.SystemFamily)
  }
  os = [pscustomobject]@{
    name = (S $os.Caption); version = (S $os.Version); build = (S $os.BuildNumber)
    displayVersion = (S $cv.DisplayVersion); architecture = (S $os.OSArchitecture)
    installUnix = (Unix $os.InstallDate)
    uptimeSecs = $(if ($boot) { [uint64]((Get-Date) - $boot).TotalSeconds } else { [uint64]0 })
    totalMemKb = (N $os.TotalVisibleMemorySize); freeMemKb = (N $os.FreePhysicalMemory)
  }
  board = [pscustomobject]@{ manufacturer = (S $bb.Manufacturer); product = (S $bb.Product); version = (S $bb.Version) }
  bios = [pscustomobject]@{
    vendor = (S $bios.Manufacturer); version = (S $bios.SMBIOSBIOSVersion); date = (Day $bios.ReleaseDate)
    smbios = ('{0}.{1}' -f (N $bios.SMBIOSMajorVersion), (N $bios.SMBIOSMinorVersion))
  }
  cpus = $cpus
  memory = [pscustomobject]@{ slots = $slots; modules = $modules }
  gpus = $gpus
  disks = $disks
  volumes = $volumes
  network = $nics
  battery = $battery
}
ConvertTo-Json -InputObject $info -Depth 6 -Compress
