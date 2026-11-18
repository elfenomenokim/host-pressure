import json, sys
d = json.load(sys.stdin)
def norm(v):
    if isinstance(v, dict):
        return {k: norm(x) for k, x in sorted(v.items())}
    if isinstance(v, list):
        return [norm(x) for x in v]
    if v is None:
        return None
    if isinstance(v, bool):
        return v
    if isinstance(v, (int, float)):
        return "<num>"
    return "<str>"
for k in ("observedAt","cpuPercent","memoryPercent","memoryUsedBytes","loadavg","uptime","uptimeSeconds"):
    if k in d:
        d[k] = f"<{type(d[k]).__name__}>"
if "disks" in d and isinstance(d["disks"], list):
    for disk in d["disks"]:
        for kk in ("usedBytes","freeBytes","percent"):
            if kk in disk:
                disk[kk] = f"<{type(disk[kk]).__name__ if disk[kk] is not None else 'None'}>"
print(json.dumps(norm(d), sort_keys=True, indent=2))
