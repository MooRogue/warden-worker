#!/usr/bin/env python3
"""Restore an exported D1 SQL dump into disposable local SQLite and check it."""
import json
from pathlib import Path
import sqlite3
import sys


def validate(source, destination):
    assert source.stat().st_size > 0, 'Empty SQL export'
    assert not destination.exists(), 'Refuse to overwrite a restore database'
    connection = sqlite3.connect(destination)
    try:
        connection.executescript(source.read_text())
        assert connection.execute('pragma quick_check').fetchone()[0] == 'ok'
        tables = connection.execute("select name from sqlite_master where type='table'").fetchall()
        assert tables, 'No tables in exported database'
        return {'sqlite_quick_check': 'ok', 'tables': len(tables), 'sql_bytes': source.stat().st_size}
    finally:
        connection.close()


if __name__ == '__main__':
    print(json.dumps(validate(Path(sys.argv[1]), Path(sys.argv[2]))))
