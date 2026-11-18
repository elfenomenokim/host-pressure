param([switch]$NoSample)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$benchRoot = Join-Path $env:TEMP ('hp-bench-' + [guid]::NewGuid().ToString('N'))
try {
  py -3 -m venv $benchRoot
  if ($LASTEXITCODE -ne 0) { throw 'venv failed' }
  $python = Join-Path $benchRoot 'Scripts\python.exe'
  & $python -m pip install --disable-pip-version-check --quiet host-pressure
  if ($LASTEXITCODE -ne 0) { throw 'pip install failed' }
  $rust = Join-Path $benchRoot 'Scripts\host-pressure.exe'
  $suffix = if ($NoSample) { ' --no-sample' } else { '' }
  $results = @()
  foreach ($edition in @('python', 'rust')) {
    $exe = if ($edition -eq 'python') { $python } else { $rust }
    $arg = $(if ($edition -eq 'python') { '-m host_pressure' } else { '' }) + $suffix
    for ($i = 0; $i -lt 3; $i++) {
      $warmArgs = @(); if ($edition -eq 'python') { $warmArgs += '-m', 'host_pressure' }; if ($NoSample) { $warmArgs += '--no-sample' }
      & $exe @warmArgs | Out-Null
    }
    $runs = @()
    $peaks = @()
    for ($i = 0; $i -lt 30; $i++) {
      $proc = New-Object System.Diagnostics.Process
      $proc.StartInfo.FileName = $exe
      $proc.StartInfo.Arguments = $arg
      $proc.StartInfo.UseShellExecute = $false
      $proc.StartInfo.CreateNoWindow = $true
      $proc.StartInfo.RedirectStandardOutput = $true
      $elapsed = Measure-Command {
        [void]$proc.Start()
        $handle = $proc.Handle
        $output = $proc.StandardOutput.ReadToEnd()
        $proc.WaitForExit()
      }
      if ($proc.ExitCode -ne 0) { throw 'benchmark command failed' }
      $runs += $elapsed.TotalMilliseconds
      $peaks += $proc.PeakWorkingSet64
      $proc.Dispose()
    }
    $results += [ordered]@{edition=$edition;command=($(if ($edition -eq 'python') {'python -m host_pressure'} else {'host-pressure.exe'}) + $suffix);milliseconds=$runs;mean_ms=($runs | Measure-Object -Average).Average;peak_working_set_bytes=$peaks}
  }
  [ordered]@{date='2026-11-17';host='Windows laptop';cpu='AMD Ryzen 7 PRO 6850U with Radeon Graphics';method='PowerShell Measure-Command with Process.Start/WaitForExit, warmup 3, runs 30';no_sample=[bool]$NoSample;runs=$results} | ConvertTo-Json -Depth 6
} finally {
  if (Test-Path -LiteralPath $benchRoot) { Remove-Item -LiteralPath $benchRoot -Recurse -Force }
}
