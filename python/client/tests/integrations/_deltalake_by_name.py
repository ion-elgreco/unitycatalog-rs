"""Appends rows to a Delta table by its name (`uc://catalog.schema.table`)
with deltalake, then prints the version of the table:

    python _deltalake_by_name.py <name> <storage options> <rows>

The storage options and the rows are JSON. test_tables.py runs this in a
process of its own: deltalake 1.6.6's first `uc://` call in a process
sometimes never returns.
"""

import json
import sys

import deltalake
import polars as pl

name, options, rows = sys.argv[1], json.loads(sys.argv[2]), json.loads(sys.argv[3])
deltalake.write_deltalake(
    name, pl.DataFrame(rows), mode="append", storage_options=options
)
print(deltalake.DeltaTable(name, storage_options=options).version())
