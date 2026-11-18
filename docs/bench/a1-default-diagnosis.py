import collections,json,statistics,subprocess,tempfile,time
from pathlib import Path
commands={"python":["/tmp/hp-venv/bin/python","-m","host_pressure"],"rust":[str(Path.home()/".local/bin/host-pressure")]}
result={"method":"30 default --json calls per edition and interval, warmup 3, isolated state directory; pause is outside measured call", "runs":{}}
for name,cmd in commands.items():
 for gap in [0,0.05]:
  state=Path(tempfile.mkdtemp(prefix="hp-default-diagnosis-"))
  args=cmd+["--state-dir",str(state),"--json"]
  for _ in range(3):subprocess.run(args,check=True,stdout=subprocess.DEVNULL)
  runs=[]
  for _ in range(30):
   time.sleep(gap)
   before=json.loads((state/"cpu_ticks_default.json").read_text())
   start=time.perf_counter();r=subprocess.run(args,check=True,capture_output=True,text=True);elapsed=(time.perf_counter()-start)*1000
   after=json.loads((state/"cpu_ticks_default.json").read_text())
   data=json.loads(r.stdout)
   runs.append({"milliseconds":elapsed,"cpuSource":data["cpuSource"],"baseline_delta_total":sum(a-b for a,b in zip(after["ticks"],before["ticks"]))})
  result["runs"][f"{name}-gap-{gap}"]={"mean_ms":statistics.mean(x["milliseconds"] for x in runs),"cpu_sources":dict(collections.Counter(x["cpuSource"] for x in runs)),"samples":runs}
print(json.dumps(result,indent=2))
