#!/usr/bin/env python3
"""Read text Sokoban boards only. Replay supplied solutions; optionally solve small boards.

Never traverses or decodes image assets. Analysis does not alter collections.
Push BFS merges equivalent player-reachable regions and prunes static dead squares.
A completed BFS proves minimum pushes, not minimum walking moves or human difficulty.
"""
import argparse
from collections import deque
import csv
import json
from math import comb, log10
from pathlib import Path
import re
import statistics
import time

DIRECTIONS = {'u': (0, -1), 'd': (0, 1), 'l': (-1, 0), 'r': (1, 0)}

def add(p, d):
    return p[0] + d[0], p[1] + d[1]

def parse_file(path):
    # Historical .sok files contain Western-European Windows text.
    data = path.read_bytes()
    try:
        text = data.decode('utf-8-sig')
    except UnicodeDecodeError:
        text = data.decode('cp1252')
    lines = text.splitlines()
    starts = []
    i = 0
    while i < len(lines):
        row = lines[i]
        if row.strip() and set(row) <= set(' #.@+$*-_pPbB'):
            start = i
            rows = []
            while i < len(lines) and lines[i].strip() and set(lines[i]) <= set(' #.@+$*-_pPbB'):
                rows.append(lines[i])
                i += 1
            raw = ''.join(rows)
            if '#' in raw and any(c in raw for c in '@+pP') and any(c in raw for c in '$*bB'):
                name = next((s.strip().lstrip(';').strip() for s in reversed(lines[:start]) if s.strip()), f'Puzzle {len(starts)+1}')
                starts.append((start, i, name, rows))
        else:
            i += 1
    levels = []
    for n, (start, end, name, rows) in enumerate(starts):
        after = starts[n+1][0] if n+1 < len(starts) else len(lines)
        metadata = lines[end:after]
        titles = [s.removeprefix('Title:').strip() for s in metadata if s.startswith('Title:')]
        if titles:
            name = titles[0]
        width, height = max(map(len, rows)), len(rows)
        floor, goals, boxes, players = set(), set(), set(), []
        for y, row in enumerate(rows):
            for x, c in enumerate(row):
                if c != '#': floor.add((x,y))
                if c in '.+*PB': goals.add((x,y))
                if c in '$*bB': boxes.add((x,y))
                if c in '@+pP': players.append((x,y))
        # Remove whitespace connected to the board's exterior, including ragged row padding.
        padded = {(x,y) for y in range(height) for x in range(width) if x >= len(rows[y]) or rows[y][x] != '#'}
        outside = {p for p in padded if p[0] in (0,width-1) or p[1] in (0,height-1)}
        todo = deque(outside)
        while todo:
            p = todo.popleft()
            for d in DIRECTIONS.values():
                q = add(p,d)
                if q in padded and q not in outside:
                    outside.add(q); todo.append(q)
        floor -= outside
        # Rare boards use boxes on goals as part of the outer border. Keep their occupied cells.
        floor |= goals | boxes | set(players)
        level = dict(collection=path.stem, number=n+1, name=name, line=start+1,
                     width=width, height=height, floor=floor, goals=goals, boxes=boxes,
                     player=players[0], valid_structure=len(players)==1 and len(boxes)==len(goals))
        candidates = []
        recording = False
        path_chars = []
        for line in metadata + ['END']:
            if re.fullmatch('[udlrUDLR]+', line.strip()) and recording:
                path_chars.append(line.strip())
            else:
                if path_chars:
                    candidates.append(''.join(path_chars)); path_chars=[]
                recording = line.strip().lower().startswith('solution')
        level['solutions'] = candidates
        levels.append(level)
    return levels

def replay(level, moves):
    boxes = set(level['boxes']); player = level['player']; pushes = off_goal = 0
    for c in moves:
        d = DIRECTIONS[c.lower()]
        q = add(player,d)
        if q not in level['floor']: return None
        if q in boxes:
            dest = add(q,d)
            if dest not in level['floor'] or dest in boxes: return None
            off_goal += q in level['goals']
            boxes.remove(q); boxes.add(dest); pushes += 1
        player = q
    if boxes != level['goals']: return None
    return dict(moves=len(moves), pushes=pushes, pushes_off_goals=off_goal)

def reachable(player, boxes, floor):
    visited={player}; todo=[player]
    for p in todo:
        for d in DIRECTIONS.values():
            q=add(p,d)
            if q in floor and q not in boxes and q not in visited:
                visited.add(q); todo.append(q)
    return visited

def solve_pushes(level, max_states=60000, seconds=4.0):
    start=time.monotonic(); floor=level['floor']; goals=level['goals']
    # Reverse-pull reachability ignores other boxes; a square outside this set
    # cannot reach any goal under any legal forward pushing arrangement.
    live=set(goals); todo=list(goals)
    for p in todo:
        for dx,dy in DIRECTIONS.values():
            prev=(p[0]-dx,p[1]-dy); support=(p[0]-2*dx,p[1]-2*dy)
            if prev in floor and support in floor and prev not in live:
                live.add(prev); todo.append(prev)
    initial=tuple(sorted(level['boxes']))
    region=reachable(level['player'],set(initial),floor)
    key=(initial,min(region))
    queue=deque([(initial,level['player'],0)]); seen={key}; expanded=0
    while queue:
        if expanded % 64 == 0 and time.monotonic()-start > seconds:
            return dict(status='time_limit', states=expanded)
        boxes,player,depth=queue.popleft(); boxset=set(boxes); expanded+=1
        if boxset == goals:
            return dict(status='optimal', pushes=depth, states=expanded, seconds=round(time.monotonic()-start,4))
        region=reachable(player,boxset,floor)
        for b in boxes:
            for dx,dy in DIRECTIONS.values():
                behind=(b[0]-dx,b[1]-dy); dest=(b[0]+dx,b[1]+dy)
                if behind not in region or dest in boxset or dest not in live: continue
                new=boxset-{b}|{dest}
                canonical=min(reachable(b,new,floor))
                key=(tuple(sorted(new)),canonical)
                if key in seen: continue
                if len(seen)>=max_states: return dict(status='state_limit', states=expanded)
                seen.add(key); queue.append((key[0],b,depth+1))
    return dict(status='unsolvable',states=expanded)

def independent_push_bound(level):
    """Minimum crate-to-goal assignment cost, respecting walls but ignoring other crates
    and the player's ability to move between pushing positions. A lower bound only.
    Limited to five crates to keep batch analysis inexpensive.
    """
    boxes=sorted(level['boxes'])
    if len(boxes)>5: return None
    distances=[]
    for goal in sorted(level['goals']):
        dist={goal:0}; queue=deque([goal])
        while queue:
            p=queue.popleft()
            for dx,dy in DIRECTIONS.values():
                prev=(p[0]-dx,p[1]-dy); support=(p[0]-2*dx,p[1]-2*dy)
                if prev in level['floor'] and support in level['floor'] and prev not in dist:
                    dist[prev]=dist[p]+1; queue.append(prev)
        distances.append(dist)
    costs={0:0}
    for box in boxes:
        next_costs={}
        for mask,cost in costs.items():
            for i,dist in enumerate(distances):
                if mask & (1<<i) or box not in dist: continue
                new=mask|(1<<i); value=cost+dist[box]
                next_costs[new]=min(next_costs.get(new,value),value)
        costs=next_costs
    return costs.get((1<<len(boxes))-1)

def record(level):
    valid=[r for s in level['solutions'] if (r:=replay(level,s))]
    best=min(valid,key=lambda x:(x['pushes'],x['moves'])) if valid else None
    cells=len(level['floor']); boxes=len(level['boxes'])
    return dict(collection=level['collection'], number=level['number'], name=level['name'],
                line=level['line'], width=level['width'], height=level['height'], floor_cells=cells,
                crates=boxes, independent_crate_push_lower_bound=independent_push_bound(level), starts_on_goal=len(level['boxes'] & level['goals']),
                valid_structure=level['valid_structure'], candidate_solutions=len(level['solutions']),
                verified_solutions=len(valid), recorded_solution=best,
                arrangement_upper_bound_log10=round(log10(comb(cells,boxes)*max(1,cells-boxes)),2))

def main():
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--solve-own',action='store_true')
    ap.add_argument('--sample-reference',action='store_true')
    ap.add_argument('--output',type=Path,default=Path('analysis/puzzle-complexity.json'))
    args=ap.parse_args(); root=Path(__file__).resolve().parent.parent
    paths=sorted((root/'shared/YASC_1_690_src').glob('*.sok'))+[root/'assets/levels.xsb']
    results=[]; summary=[]
    for path in paths:
        levels=parse_file(path)
        if path.name=='levels.xsb':
            witnesses=json.loads((root/'tests/solutions.json').read_text())
            assert len(levels)==len(witnesses)
            for level,witness in zip(levels,witnesses): level['solutions']=[witness]
        entries=[record(l) for l in levels]
        if args.solve_own and path.name=='levels.xsb':
            for level,entry in zip(levels,entries):
                entry['solver']=solve_pushes(level,seconds=8.0)
                print(level['name'],entry['solver'],flush=True)
        if args.sample_reference and path.name!='levels.xsb':
            eligible=[(l,e) for l,e in zip(levels,entries) if len(l['boxes'])<=5 and len(l['floor'])<=50]
            for level,entry in eligible[:3]:
                entry['solver']=solve_pushes(level,seconds=3.0)
                print(path.name,level['name'],entry['solver'],flush=True)
        solutions=[e['recorded_solution'] for e in entries if e['recorded_solution']]
        def stats(values):
            return {'min':min(values),'median':statistics.median(values),'max':max(values)} if values else None
        summary.append(dict(collection=path.stem,boards=len(entries),
            crates=stats([e['crates'] for e in entries]),floor_cells=stats([e['floor_cells'] for e in entries]),
            recorded_pushes=stats([s['pushes'] for s in solutions]),recorded_moves=stats([s['moves'] for s in solutions]),
            solved_records=len(solutions), solution_candidates=sum(e['candidate_solutions'] for e in entries),
            verified_solutions=sum(e['verified_solutions'] for e in entries),
            solutions_pushing_off_goal=sum(s['pushes_off_goals']>0 for s in solutions)))
        results.extend(entries)
        print(path.name,summary[-1],flush=True)
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps({'method':'Static board metrics and replay-verified existing solutions. Recorded paths are not claimed optimal. Optional bounded push BFS proves push optimality only when status=optimal.', 'summary':summary,'levels':results},indent=2)+'\n')
    with args.output.with_suffix('.csv').open('w') as f:
        w=csv.writer(f);w.writerow(['collection','boards','crates_min','crates_median','crates_max','floor_median','verified_boards','recorded_pushes_min','recorded_pushes_median','recorded_pushes_max'])
        for s in summary:
            w.writerow([s['collection'],s['boards'],s['crates']['min'],s['crates']['median'],s['crates']['max'],s['floor_cells']['median'],s['solved_records'],*[s['recorded_pushes'][k] if s['recorded_pushes'] else '' for k in ['min','median','max']]])

if __name__=='__main__': main()
