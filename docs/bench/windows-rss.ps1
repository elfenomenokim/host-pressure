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
  & $python -c "import base64; exec(base64.b64decode('aW1wb3J0IGN0eXBlcyxjdHlwZXMud2ludHlwZXMgYXMgdyxqc29uLHN1YnByb2Nlc3Msc3lzCmNsYXNzIFBNQyhjdHlwZXMuU3RydWN0dXJlKToKIF9maWVsZHNfPVsoImNiIix3LkRXT1JEKSwoIlBhZ2VGYXVsdENvdW50Iix3LkRXT1JEKV0rWyhuLGN0eXBlcy5jX3NpemVfdCkgZm9yIG4gaW4gWyJQZWFrV29ya2luZ1NldFNpemUiLCJXb3JraW5nU2V0U2l6ZSIsIlF1b3RhUGVha1BhZ2VkUG9vbFVzYWdlIiwiUXVvdGFQYWdlZFBvb2xVc2FnZSIsIlF1b3RhUGVha05vblBhZ2VkUG9vbFVzYWdlIiwiUXVvdGFOb25QYWdlZFBvb2xVc2FnZSIsIlBhZ2VmaWxlVXNhZ2UiLCJQZWFrUGFnZWZpbGVVc2FnZSJdXQpnZXQ9Y3R5cGVzLldpbkRMTCgicHNhcGkiKS5HZXRQcm9jZXNzTWVtb3J5SW5mbwpnZXQuYXJndHlwZXM9W3cuSEFORExFLGN0eXBlcy5QT0lOVEVSKFBNQyksdy5EV09SRF07Z2V0LnJlc3R5cGU9dy5CT09MCnJlc3VsdD17fQpmb3IgbmFtZSxjbWQgaW4gWygicHl0aG9uIixbc3lzLmV4ZWN1dGFibGUsIi1tIiwiaG9zdF9wcmVzc3VyZSJdKSwoInJ1c3QiLFtzeXMuYXJndlsxXV0pXToKIHByb2M9c3VicHJvY2Vzcy5Qb3BlbihjbWQsc3Rkb3V0PXN1YnByb2Nlc3MuUElQRSkKIG91dCxfPXByb2MuY29tbXVuaWNhdGUoKTtwbWM9UE1DKCk7cG1jLmNiPWN0eXBlcy5zaXplb2YocG1jKQogaWYgbm90IGdldCh3LkhBTkRMRShpbnQocHJvYy5faGFuZGxlKSksY3R5cGVzLmJ5cmVmKHBtYykscG1jLmNiKTpyYWlzZSBjdHlwZXMuV2luRXJyb3IoKQogcmVzdWx0W25hbWVdPXsicGVha193b3JraW5nX3NldF9ieXRlcyI6cG1jLlBlYWtXb3JraW5nU2V0U2l6ZSwib3V0cHV0IjpvdXQuZGVjb2RlKCkuc3RyaXAoKX0KcHJpbnQoanNvbi5kdW1wcyhyZXN1bHQsaW5kZW50PTIpKQo='))" $rust

} finally {
  if (Test-Path -LiteralPath $benchRoot) { Remove-Item -LiteralPath $benchRoot -Recurse -Force }
}
