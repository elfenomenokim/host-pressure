# host-pressure

[English](https://github.com/elfenomenokim/host-pressure/blob/main/README.md)

이 머신이 지금 무엇을 하고 있는지, 여유가 있는지 한 줄로 읽습니다.

```console
$ pipx install host-pressure
$ host-pressure
CPU 15% MEM 76% SWAP 1.4G pressure normal
```

서브프로세스를 띄우지 않고, 의존성도 없고, 데몬도 두지 않습니다. 값은 커널에서 직접 읽습니다. macOS는 ctypes로 Mach와 `sysctl`, Linux는 `/proc`, Windows는 `kernel32`입니다.

## 왜 쓰는가

- **숫자가 아니라 판정이 필요할 때.** `host-pressure check`는 `ok`, `busy`, `overloaded` 가운데 하나와 그렇게 판단한 근거를 함께 출력하고 0, 1, 2로 끝납니다. `host-pressure wait --until ok --timeout 120`은 머신에 여유가 생길 때까지 스크립트나 에이전트를 세워 둡니다.
- **psutil이 주지 않는 세 가지.** macOS의 메모리 압력 등급(활성 상태 보기가 보여 주는 커널 신호), Linux의 CPU·메모리·IO 압력 정체 정보(`/proc/pressure`), macOS의 열 상태입니다. psutil 7.2는 메모리 비율과 스왑 바이트, 센서별 온도를 주지만 압력 등급도 PSI도 열 상태도 주지 않습니다.
- **CPU는 표본 두 개가 필요하다는 점을 숨기지 않습니다.** 사용률은 카운터 두 번을 뺀 값이라 한 번의 읽기로는 나오지 않습니다. 직전 카운터를 소비자 id마다 보관하므로 상태 바에서는 기다릴 것이 없고, `--sample 0.5`는 한 호출 안에서 두 번 읽고 그 상태는 건드리지 않습니다.
- **상태 바에는 한 줄이면 됩니다.** `--format 'CPU {cpu} MEM {mem}'`는 상태 바에 필요한 값만 출력하고, 각 자리표시자는 단위를 이미 포함합니다.
- **두 번 설치할 것이 없습니다.** 표준 라이브러리만 씁니다. `pipx`나 `uvx`, 또는 열 줄짜리 import면 충분합니다.

```console
$ host-pressure --all
CPU 13% MEM 76% SWAP 1.4G pressure normal LOAD 2.94 3.49 3.53 DISK / 44% THERM nominal
```

압력 정체 정보를 제공하는 Linux 서버에서는 같은 명령이 이렇게 나옵니다.

```console
$ host-pressure --all
CPU 57% MEM 12% SWAP 0B pressure normal LOAD 0.92 0.52 0.57 DISK / 66% PSI cpu 1.5 mem 0.0 io 0.1
```

## 설치

```console
$ pipx install host-pressure     # 권장: 격리 설치, PATH에 등록
$ uvx host-pressure              # 설치 없이 실행
$ pip install host-pressure      # 기존 환경 안에서
```

Python 3.9 이상. 의존성도, 빌드 단계도 wheel 말고는 없습니다.

## 판본

하나의 PyPI 이름으로 두 판본을 함께 냅니다.

- **Python**(`src/host_pressure/`): 순수 표준 라이브러리. Python 3.9 이상이면 어디서든 `import host_pressure`가 됩니다.
- **Rust**(`rust/`): 같은 CLI, 같은 `--json` 키와 종료 코드. `pipx install host-pressure`는 빠른 Rust 바이너리를 `host-pressure` 명령으로 설치하고, 순수 Python 패키지는 그대로 import됩니다(maturin `bindings = "bin"` + `python-source = "src"`. `cargo install --path rust`도 됩니다).

CPU 기준값 상태 파일은 공유합니다. 같은 경로(`$XDG_STATE_HOME/host-pressure/cpu_ticks_<consumer>.json`), 같은 JSON 형식이라 두 판본이 하나의 `--consumer` id 아래서 번갈아 쓸 수 있습니다. `bash rust/parity.sh`로 같은 플래그의 동등성을 확인합니다(순간마다 바뀌는 숫자는 정규화해서 비교).

2026-11-17 측정, 20회 반복, `--no-sample` 기준 경로:

| 호스트 | Python | Rust | 빨라짐 |
|---|---|---|---|
| M5 macOS arm64 | 74.5 ms | 9.3 ms | 8.0배 |
| A1 Ubuntu 24.04 arm64 | 73.7 ms | 2.1 ms | 35배 |

`--sample 0.1`은 sleep이 지배적이라 두 판본이 비슷합니다(커널이 CPU 카운터를 묶어서 공개하므로 짧은 구간이 최대 1초까지 늘어납니다). 릴리스 바이너리: 706 KB(macOS arm64), 850 KB(Linux arm64). 피크 RSS: Rust 약 2 MB, Python 약 13-16 MB. Windows Rust는 확인하지 않았습니다(노트북에 툴체인이 없음). Windows Python판은 앞서 확인했습니다.

## 사용법

### 한 줄

```console
$ host-pressure
CPU 15% MEM 76% SWAP 1.4G pressure normal
```

플랫폼이 답할 수 없는 값은 `CPU`, `MEM`, `SWAP` 자리에 `n/a`, 압력은 `normal`, `warning`, `critical`, `unknown` 가운데 하나로 나옵니다. 빈칸을 그럴듯한 숫자로 채우지 않습니다.

### 한 번에 전부

```console
$ host-pressure --all
CPU 13% MEM 76% SWAP 1.4G pressure normal LOAD 2.94 3.49 3.53 DISK / 44% THERM nominal
```

`--all`은 부하 평균, `--disk`로 지정한 마운트마다 `DISK <마운트> <퍼센트>`, 열 상태, 배터리, 커널이 제공하는 PSI 정체율을 뒤에 붙입니다. `--disk /Volumes/work --disk /`처럼 두 번 주면 두 마운트를 읽고, 첫 번째가 `{disk}` 자리표시자에도 들어갑니다.

### 상태 바용 템플릿

```console
$ host-pressure --format 'CPU {cpu} MEM {mem} | load {load1}'
CPU 8% MEM 76% | load 2.94
```

```console
$ host-pressure --format 'cpu {cpu} mem {mem} swap {swap} {level}'
cpu 20% mem 73% swap 1.4G ok
```

자리표시자 전체 목록은 `host-pressure --format-help`로 봅니다. 값마다 단위가 붙어 있으므로(`{cpu}` → `16%`, `{swap}` → `1.5G`) 템플릿에 `%`를 또 쓰지 않습니다. 없는 이름을 쓰면 빈칸이 아니라 오류로 끝납니다.

### 기계가 읽는 형식

```console
$ host-pressure --json
{
  "observedAt": "2026-11-17T08:24:55.523683+00:00",
  "os": "darwin",
  "hostname": "<host name>",
  "ncpu": 18,
  "cpuPercent": 14.3,
  "cpuSource": "baseline",
  "memoryPercent": 75.6,
  "memoryTotalBytes": 68719476736,
  "memoryUsedBytes": 51943424000,
  "swapBytes": 1516306432,
  "swapTotalBytes": 3221225472,
  "pressure": "normal",
  "pressureSource": "macos",
  ...
  "verdict": {
    "level": "ok",
    "reasons": []
  }
}
```

`cpuSource`는 CPU 값이 어디서 왔는지 밝힙니다. `baseline`이면 같은 소비자 id로 직전에 호출했을 때의 카운터 차이, `sample`이면 이번 호출 안에서 잰 구간이고, 둘 다 아니면 `null`입니다. `pressureSource`는 압력을 결정한 신호(`macos`, `psi-avg60`, `memavailable`, `memoryload`)입니다.

### 판정과 종료 코드

```console
$ host-pressure check
ok
$ echo $?
0
```

압력이 걸리면 등급과 근거가 한 줄에 나옵니다.

```console
$ host-pressure check --memory-busy 10
busy: memory 75.7% >= 10%
$ echo $?
1
```

`overloaded`는 2로 끝납니다. `--json`은 같은 결과를 `{"level": ..., "reasons": [...]}`로 출력합니다. 근거 문장은 읽은 값과 기준값만 적습니다.

### 여유가 생길 때까지 기다리기

```console
$ host-pressure wait --until ok --timeout 120
ok
```

`wait`는 `--interval`초(기본 2초)마다 다시 재고, 매번 `--sample` 구간(기본 0.2초)을 새로 잡습니다. 등급이 `--until`과 같거나 더 좋아지면 바로 끝납니다. 시간이 다 되면 75(EX_TEMPFAIL)로 끝나고 마지막 판정을 stderr에 남기므로, 부르는 쪽이 "아직 바쁨"과 "고장"을 구별할 수 있습니다.

```console
$ host-pressure wait --until busy --timeout 30 && make -j8
```

### 계속 보기

```console
$ host-pressure --watch 2
CPU 45% MEM 73% SWAP 1.4G pressure normal
CPU 21% MEM 74% SWAP 1.4G pressure normal
CPU 16% MEM 74% SWAP 1.4G pressure normal
```

`--watch`는 지정한 초마다 같은 줄을 찍고, 인터럽트하면 끝납니다. `--json`과 함께 쓰면 JSON 객체를 한 줄씩 출력하므로 JSONL을 읽는 도구에 그대로 넘길 수 있습니다.

### CPU: 기준값 아니면 표본

CPU는 카운터를 두 번 읽어야 합니다. 방법이 둘이고, 도구는 어느 쪽을 썼는지 숨기지 않습니다.

| 방법 | 플래그 | 비용 | 쓰는 자리 |
|---|---|---|---|
| 기준값 | 기본 | 없음. stat 한 번과 작은 파일 쓰기 한 번 | 상태 바, cron, N초마다 부르는 데몬 |
| 표본 | `--sample 0.5` | 지정한 시간만큼 한 번 sleep | 지금 당장 숫자를 보여 줘야 하는 한 번 실행 |
| 기다리지 않음 | `--no-sample` | 없음. 기준값이 없으면 `n/a` | 지연이 엄격한 자리 |

기준값은 `--consumer` id별로 `$XDG_STATE_HOME/host-pressure` 아래에 따로 저장되므로 두 도구가 서로의 구간을 덮어쓰지 않습니다. 새 소비자의 첫 호출은 `n/a`이고, 아무 플래그 없는 `host-pressure`는 이 빈칸을 한 번만 0.3초 표본으로 채웁니다. 새로 설치한 직후의 첫 실행이 0.3초쯤 걸리는 이유입니다.

## 옵션

| 플래그 | 뜻 | 기본값 |
|---|---|---|
| `--json` | 읽은 값 전체를 JSON으로 출력 | 꺼짐 |
| `--format TEMPLATE` | 지정한 형식으로 한 줄 출력 | 꺼짐 |
| `--format-help` | 템플릿 자리표시자 목록 | — |
| `--all` | 부하·디스크·열·배터리·PSI를 덧붙임 | 꺼짐 |
| `--watch N` | N초마다 반복 | 꺼짐 |
| `--disk MOUNT` | 이 마운트도 읽음, 반복 가능 | `/` |
| `--sample N` | 이번 호출에서 CPU 표본 두 개를 잼 | 꺼짐 |
| `--no-sample` | CPU 표본을 기다리지 않음 | 꺼짐 |
| `--consumer ID` | 이 호출자의 CPU 기준값 이름 | `default` |
| `--state-dir DIR` | CPU 기준값을 두는 곳 | `$XDG_STATE_HOME/host-pressure` |
| `--config PATH` | 기준값 파일 | `$XDG_CONFIG_HOME/host-pressure/config.json` |
| `--cpu-busy`, `--cpu-overloaded` | CPU 기준선, 퍼센트 | 80 / 95 |
| `--memory-busy`, `--memory-overloaded` | 메모리 기준선, 퍼센트 | 85 / 95 |
| `--load-busy`, `--load-overloaded` | 코어당 부하 평균 | 1 / 2 |
| `--disk-busy`, `--disk-overloaded` | 디스크 사용률 기준선, 퍼센트 | 90 / 97 |
| `--swap-busy-gb`, `--swap-overloaded-gb` | 사용 스왑 기준선, GiB | 꺼짐 |

`wait`는 `--until ok|busy|overloaded`, `--timeout SECONDS`(120), `--interval SECONDS`(2)를 더 받습니다.

## 기준값 바꾸기

한 번만 바꾸려면 위 플래그를, 셸 전체에서 바꾸려면 `HOST_PRESSURE_<KEY>`를, 계속 바꾸려면 `$XDG_CONFIG_HOME/host-pressure/config.json`을 씁니다.

```json
{
  "cpu_busy": 70,
  "memory_busy": 80,
  "load_overloaded": 3,
  "disk_busy": 85,
  "swap_overloaded_gb": 8
}
```

키 이름은 플래그 이름을 대문자 스네이크 표기로 옮긴 것입니다: `cpu_busy`, `cpu_overloaded`, `memory_busy`, `memory_overloaded`, `load_busy`, `load_overloaded`, `disk_busy`, `disk_overloaded`, `swap_busy_gb`, `swap_overloaded_gb`, `psi_cpu_busy`, `psi_cpu_overloaded`, `psi_memory_busy`, `psi_memory_overloaded`, `psi_io_busy`, `psi_io_overloaded`. 키를 잘못 쓰면 조용히 무시하지 않고 쓸 수 있는 키 목록을 보여 주며 멈춥니다.

두 규칙은 고정입니다. 커널이 이미 판단한 값이기 때문입니다. macOS 메모리 압력 등급은 `warning`이면 busy, `critical`이면 overloaded이고, 열 상태는 `fair`면 busy, `serious`나 `critical`이면 overloaded입니다.

| 환경 변수 | 뜻 |
|---|---|
| `HOST_PRESSURE_CONFIG` | 기준값 파일 경로 |
| `HOST_PRESSURE_STATE_DIR` | CPU 기준값 디렉터리 |
| `HOST_PRESSURE_<KEY>` | 기준값 하나, 예: `HOST_PRESSURE_CPU_BUSY=70` |

## 파이썬에서

```python
import host_pressure

host_pressure.read()                    # 읽은 값 전체를 dict로
host_pressure.read(sample=0.5)          # 이번 호출 안 0.5초 구간으로 CPU를 잼
host_pressure.read(mounts=["/", "/data"])
host_pressure.verdict()                 # {"level": "ok", "reasons": []}
host_pressure.verdict(thresholds=host_pressure.Thresholds(memory_busy=50))
host_pressure.sample_cpu_percent(0.2)   # CPU만, 그것도 한 번만
```

`read()`는 프로세스를 띄우지 않고, `sample`을 넘기지 않으면 잠들지도 않습니다. 같은 계정에서 여러 프로그램이 부른다면 `consumer_id="my-tool"`을 넘기십시오. id마다 기준값 파일과 구간이 하나씩 생깁니다.

```python
>>> import host_pressure
>>> host_pressure.verdict(thresholds=host_pressure.Thresholds(memory_busy=50))
{'level': 'busy', 'reasons': ['memory 72.8% >= 50%']}
```

## 플랫폼별로 읽는 곳

| 값 | macOS | Linux | Windows |
|---|---|---|---|
| CPU | `host_statistics`(Mach) | `/proc/stat` | `GetSystemTimes` |
| 메모리 | 활성 상태 보기 공식: 앱 + wired + 압축 | `MemAvailable` | `GlobalMemoryStatusEx` |
| 스왑 | `vm.swapusage` | `SwapTotal - SwapFree` | 사용 중인 페이지 파일(`NtQuerySystemInformation`) |
| 압력 | `kern.memorystatus_vm_pressure_level` | `/proc/pressure`(PSI), 없으면 `MemAvailable` | 메모리 사용률 |
| 부하 평균, CPU 수 | `getloadavg`, `os.cpu_count` | 같음 | CPU 수만 |
| 열 | `NSProcessInfo.thermalState` | 가장 뜨거운 `/sys/class/thermal` 구역 | — |
| 배터리 | IOKit 전원 소스 | `/sys/class/power_supply` | `GetSystemPowerStatus` |
| 디스크 | `statvfs` | `statvfs` | `GetDiskFreeSpaceExW` |
| 가동 시간 | `kern.boottime` | `/proc/uptime` | `GetTickCount64` |

## 한계

- **확인한 환경**은 macOS 27 arm64(18코어 데스크톱)와 Ubuntu 24.04 arm64(2 vCPU 서버), Python 3.9와 3.12입니다. 위의 모든 예시는 이 두 머신에서 실제로 나온 출력입니다.
- **Windows**에서는 부하 평균과 발열 값이 없습니다. 압력은 메모리 사용률로 판단합니다. Windows 11 노트북에서 메모리·페이지 파일 값을 Windows 자체 값과 맞춰 봤습니다.
- **배터리도 확인하지 않았습니다.** 두 테스트 머신 모두 배터리가 없어 "배터리 없음" 경로만 지나갔습니다. 실제 퍼센트를 돌려주는 경로(IOKit, `/sys/class/power_supply`, `GetSystemPowerStatus`)는 눈으로 보지 못했습니다.
- **macOS 열 상태**는 Objective-C 런타임을 ctypes로 거쳐 `NSProcessInfo.thermalState`를 읽습니다. 공개 API지만 그 다리는 우리가 놓은 것이고, Foundation을 불러오지 못하면 null이 됩니다.
- **Linux 열 상태**는 커널이 노출하는 가장 뜨거운 구역입니다. 가상 머신은 대개 노출하지 않으므로 그 자리에서는 null입니다.
- **짧은 표본은 기다릴 수 있습니다.** 커널은 CPU 카운터를 묶어서 공개합니다(Apple Silicon에서는 대략 1초에 한 번). 그래서 그보다 짧은 `--sample` 구간은 카운터가 움직일 때까지 최대 1초까지 늘어납니다. 기다리기 싫으면 `--no-sample`을 쓰십시오. 대신 `n/a`가 나옵니다.
- **기준값 구간은 호출 사이의 간격입니다.** 같은 초 안에 두 번 실행하면 잰 것이 거의 없습니다. 그 간격의 값은 원래 잡음입니다. 구간을 정확히 정해야 하면 `--sample`을 쓰십시오.
- **macOS에서 스왑은 압력 신호가 아닙니다.** 가벼운 사용에도 며칠씩 기가바이트 단위 스왑에 머뭅니다. 그래서 스왑 기준선은 기본으로 꺼져 있고, 신뢰할 신호는 메모리 압력 등급입니다.

## 에이전트에게

`AGENTS.md`는 AI 에이전트가 이 도구를 언제 꺼내야 하는지, 어떤 질문에 어떤 명령이 답하는지, 종료 코드를 어떻게 읽는지, 하지 말아야 할 일이 무엇인지 적어 둡니다.

MIT 라이선스.
