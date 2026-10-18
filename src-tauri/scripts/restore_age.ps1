$p = Get-ComputerRestorePoint | Sort-Object SequenceNumber -Descending | Select-Object -First 1
$age = $null
if ($p) {
  $created = [Management.ManagementDateTimeConverter]::ToDateTime($p.CreationTime)
  $age = [int]((Get-Date) - $created).TotalMinutes
}
ConvertTo-Json -InputObject ([pscustomobject]@{ ageMinutes = $age }) -Compress
