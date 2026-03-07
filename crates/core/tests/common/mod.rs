pub const AXIOM_FILE: &str = "\
---
id: \"n-4a7b2c\"
title: \"Conservation of Energy\"
type: \"axiom\"
tags:
  - \"physics\"
  - \"well-established\"
status: \"current\"
status_updated_at: \"2026-03-04T14:30:00Z\"
status_updated_by: \"github-username\"
created_at: \"2026-02-15T10:00:00Z\"
created_by: \"github-username\"
dependencies: []
---

# Conservation of Energy

Energy cannot be created or destroyed in an isolated system.
";

pub const DEDUCTION_FILE: &str = "\
---
id: \"n-7c1d3e\"
title: \"Orbital Mechanics Follow from Newton's Laws\"
type: \"deduction\"
tags:
  - \"physics\"
  - \"well-established\"
status: \"stale\"
status_updated_at: \"2026-03-04T15:00:00Z\"
status_updated_by: \"system\"
stale_sources:
  - node_id: \"n-4a7b2c\"
    changed_at: \"2026-03-04T14:30:00Z\"
created_at: \"2026-02-20T09:00:00Z\"
created_by: \"github-username\"
dependencies:
  - node_id: \"n-4a7b2c\"
    annotation:
      label: \"requires\"
  - node_id: \"n-3f8a1d\"
    annotation:
      label: \"supports\"
relation: \"(n-4a7b2c AND n-3f8a1d)\"
---

# Orbital Mechanics Follow from Newton's Laws

Given conservation of energy and Newton's gravitational law...
";
