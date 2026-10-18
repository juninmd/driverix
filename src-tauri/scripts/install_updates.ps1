$ids = @((ConvertFrom-Json $env:DRIVERIX_INPUT).updateIds)
$session = New-Object -ComObject Microsoft.Update.Session
$result = $session.CreateUpdateSearcher().Search("IsInstalled=0 and Type='Driver' and IsHidden=0")
$coll = New-Object -ComObject Microsoft.Update.UpdateColl
$out = New-Object System.Collections.ArrayList
foreach ($u in $result.Updates) {
  if ($ids -notcontains [string]$u.Identity.UpdateID) { continue }
  # An unaccepted EULA is reported, never accepted on the user's behalf.
  if (-not $u.EulaAccepted) {
    [void]$out.Add([pscustomobject]@{ updateId = [string]$u.Identity.UpdateID; resultCode = 0; hresult = 0; rebootRequired = $false; note = 'eula' })
    continue
  }
  [void]$coll.Add($u)
}
if ($coll.Count -gt 0) {
  $dl = $session.CreateUpdateDownloader()
  $dl.Updates = $coll
  [void]$dl.Download()
  $inst = $session.CreateUpdateInstaller()
  $inst.Updates = $coll
  $res = $inst.Install()
  for ($i = 0; $i -lt $coll.Count; $i++) {
    $r = $res.GetUpdateResult($i)
    [void]$out.Add([pscustomobject]@{
      updateId = [string]$coll.Item($i).Identity.UpdateID
      resultCode = [int]$r.ResultCode
      hresult = [int]$r.HResult
      rebootRequired = [bool]$r.RebootRequired
      note = ''
    })
  }
}
ConvertTo-Json -InputObject @($out) -Depth 3 -Compress
