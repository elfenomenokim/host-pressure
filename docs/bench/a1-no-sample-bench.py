import json,platform,statistics,subprocess,tempfile,time
from pathlib import Path
commands={"python":["/tmp/hp-venv/bin/python","-m","host_pressure"],"rust":[str(Path.home()/".local/bin/host-pressure")]}
result={"date":"2026-11-17","host":"A1","os":platform.platform(),"method":"perf_counter subprocess.run, --no-sample, warmup 3, runs 30; separate /usr/bin/time -v for peak RSS","runs":{}}
for name,cmd in commands.items():
 cmd=cmd+["--no-sample"]
 for _ in range(3):subprocess.run(cmd,check=True,stdout=subprocess.DEVNULL)
 times=[]
 for _ in range(30):
  start=time.perf_counter();subprocess.run(cmd,check=True,stdout=subprocess.DEVNULL);times.append((time.perf_counter()-start)*1000)
 mem=subprocess.run(["/usr/bin/time","-v",*cmd],text=True,capture_output=True,check=True)
 result["runs"][name]={"command":cmd,"milliseconds":times,"mean_ms":statistics.mean(times),"time_stdout":mem.stdout,"time_stderr":mem.stderr}
print(json.dumps(result,indent=2))
