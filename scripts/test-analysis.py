#!/usr/bin/env python3
"""Small independent fixtures for the analysis solver and saved witness validation."""
from pathlib import Path
import json
import runpy
import tempfile
import unittest
ROOT=Path(__file__).resolve().parent.parent
api=runpy.run_path(str(ROOT/'scripts/analyze-levels.py'))

class AnalysisChecks(unittest.TestCase):
    def board(self, rows):
        with tempfile.TemporaryDirectory() as d:
            path=Path(d)/'fixture.xsb'; path.write_text(rows)
            return api['parse_file'](path)[0]

    def test_known_minimum_and_no_pulling(self):
        level=self.board('#######\n#@ $ .#\n#######\n')
        result=api['solve_pushes'](level)
        self.assertEqual((result['status'],result['pushes']),('optimal',2))
        self.assertEqual(api['replay'](level,'rrr')['pushes'],2)
        trapped=self.board('#######\n#$@  .#\n#######\n')
        self.assertEqual(api['solve_pushes'](trapped)['status'],'unsolvable')

    def test_witnesses_and_bounds(self):
        data=json.loads((ROOT/'analysis/original-prototypes.json').read_text())
        levels=api['parse_file'](ROOT/'analysis/original-prototypes.xsb')
        self.assertEqual(len(levels),len(data['levels']))
        for level,record in zip(levels,data['levels']):
            self.assertIsNotNone(api['replay'](level,record['witness']))
            result=api['solve_pushes'](level)
            self.assertEqual(result['status'],'optimal')
            self.assertEqual(result['pushes'],record['optimal_pushes'])
            limited=api['solve_pushes'](level,max_states=1)
            self.assertEqual(limited['status'],'state_limit')

    def test_collection_metrics_agree_with_records(self):
        data=json.loads((ROOT/'analysis/puzzle-complexity.json').read_text())
        for summary in data['summary']:
            records=[r for r in data['levels'] if r['collection']==summary['collection']]
            self.assertEqual(len(records),summary['boards'])
            self.assertEqual(sum(r['recorded_solution'] is not None for r in records),summary['solved_records'])
            self.assertTrue(all(r['valid_structure'] for r in records))

if __name__=='__main__': unittest.main()
