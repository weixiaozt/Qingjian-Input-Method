"""Read the public code/target cases and report candidate ranks; never writes to Qingjian."""
from pathlib import Path
import argparse,json,subprocess,statistics

lab=Path(__file__).resolve().parents[1]
parser=argparse.ArgumentParser()
parser.add_argument('--exe',type=Path,default=lab/'target/release/wubi-lab.exe')
parser.add_argument('--output',type=Path,default=lab/'评测结果.json')
args=parser.parse_args()
cases=json.loads((lab/'tests/cases.json').read_text(encoding='utf-8'))
run=subprocess.run([str(args.exe.resolve()),'--root',str(lab),'--jsonl'],input=''.join(json.dumps({'codes':c['codes']})+'\n' for c in cases),encoding='utf-8',capture_output=True,timeout=120)
if run.returncode:raise SystemExit(run.stderr or f'Process exited {run.returncode}')
responses=[json.loads(line) for line in run.stdout.splitlines()]
if len(responses)!=len(cases):raise SystemExit('Response count does not match input')
for case,result in zip(cases,responses):
    if 'error' in result:raise SystemExit(result['error'])
    candidates=result['candidates'];texts=[c['text'] for c in candidates]
    for candidate in candidates:
        assert ''.join(s['code'] for s in candidate['segments'])+candidate['pending']==case['codes']
    case.update(rank=texts.index(case['text'])+1 if case['text'] in texts else None,first=texts[0] if texts else '',elapsed_ms=result['elapsed_ms'])
summary={'cases':len(cases),'top1':sum(c['rank']==1 for c in cases),'top12':sum(c['rank'] is not None for c in cases),'median_ms':statistics.median(c['elapsed_ms'] for c in cases),'max_ms':max(c['elapsed_ms'] for c in cases)}
args.output.write_text(json.dumps({'summary':summary,'cases':cases},ensure_ascii=False,indent=2),encoding='utf-8')
print(json.dumps(summary,ensure_ascii=False))
