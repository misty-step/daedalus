# PR Context: gate-regression

The PR wraps the local gate to make logs shorter. It accidentally treats skipped pytest as success and dereferences a missing config key on projects without optional coverage settings. This task checks whether the master preserves two different supported defects instead of collapsing everything into one generic gate complaint.
