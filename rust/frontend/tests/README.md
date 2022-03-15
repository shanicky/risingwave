# Plan Test Runner

This module contains a testing tool for binder, planner and optimizer.
Given a sequence of SQL queries as the input, the test runner will check
the logical operator tree if any, and the physical operator tree if any.

Just the same as a normal Rust unit test, you can independently run the test runner
via function `run_all_test_files()` in `plan_test_runner.rs`.

The test data in YAML format is organized under `tests/testdata` folder.

```yaml
- sql: select * from t
  binder_error: "Item not found: relation \"t\""
```

This is a simple test case that validates the binder's behavior on an illegal SQL.

```yaml
- sql: |
    create table t (v1 bigint, v2 double precision);
    select * from t;
  logical_plan: |
    LogicalProject { exprs: [$0, $1, $2], expr_alias: [None, None, None] }
      LogicalScan { table: "t", columns: ["_row_id", "v1", "v2"] }
```

If the SQL is valid, then test runner will compare the generated logical operator tree
with the expected tree.