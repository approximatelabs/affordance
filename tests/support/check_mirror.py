"""Regression checks for the Python inspection interoperability fixture."""
import json
import tempfile
import unittest
from affordance_mirror import Mirror, apply_line

class MirrorChecks(unittest.TestCase):
    def test_distinct_names_never_share_a_file(self):
        with tempfile.TemporaryDirectory() as directory:
            for agent, value in [("a/b", 1), ("a__b", 2), ("a/☃", 3)]:
                apply_line(directory, json.dumps({"id": agent, "version": 1,
                           "changes": [{"key": "n", "set": value}]}))
            mirror = Mirror(directory)
            self.assertEqual(mirror.ids(), ["a/b", "a/☃", "a__b"])
            self.assertEqual([mirror[agent]["n"] for agent in mirror.ids()], [1, 3, 2])

    def test_nested_list_index_updates(self):
        with tempfile.TemporaryDirectory() as directory:
            apply_line(directory, json.dumps({"id": "a", "version": 1,
                       "changes": [{"key": "items", "set": [[1]]}]}))
            apply_line(directory, json.dumps({"id": "a", "version": 2,
                       "changes": [{"key": "items", "path": ["0"], "from": 1,
                                    "items": [2], "length": 2}]}))
            self.assertEqual(Mirror(directory)["a"]["items"], [[1, 2]])

if __name__ == "__main__":
    unittest.main()
