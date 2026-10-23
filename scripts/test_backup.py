import importlib.util
from pathlib import Path
import sqlite3
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('backup', Path(__file__).with_name('validate-backup.py'))
backup = importlib.util.module_from_spec(spec)
spec.loader.exec_module(backup)


class BackupTests(unittest.TestCase):
    def check_dump(self, sql):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'dump.sql').write_text(sql)
            return backup.validate(root / 'dump.sql', root / 'restore.sqlite')

    def test_restore_valid_sql(self):
        result = self.check_dump('CREATE TABLE example(id INTEGER PRIMARY KEY, data TEXT); INSERT INTO example VALUES(1, "example");')
        self.assertEqual(result['sqlite_quick_check'], 'ok')
        self.assertEqual(result['tables'], 1)

    def test_empty_rejected(self):
        with self.assertRaises(AssertionError):
            self.check_dump('')

    def test_missing_schema_rejected(self):
        with self.assertRaises(AssertionError):
            self.check_dump('-- not a database')

    def test_invalid_sql_rejected(self):
        with self.assertRaises(sqlite3.Error):
            self.check_dump('this is not SQL;')
