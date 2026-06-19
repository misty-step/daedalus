const product = {
  task: 'pr-review-suite',
  arena: 'pr-review-master-v0',
  run: 'runs/20260612T220412Z-search-pr-review-master',
  contract: 'deliveries/pr-review/contract.toml',
  composition: '4a73f1fd213aa1a5',
};

function tag(kind, text) {
  return `<span class="tag ${kind}">${text}</span>`;
}

function shell(active, body, actions = '<button class="primary">Open packet</button><button>Replay</button>') {
  const nav = ['Dossier', 'Matrix', 'Trace', 'Swarm', 'Gate', 'Hypotheses']
    .map((item) => `<a href="#0" class="${item === active ? 'active' : ''}">${item}</a>`)
    .join('');
  return `<div class="screen">
    <header class="topbar">
      <div class="brand"><b>Daedalus</b><small>task foundry - ${product.task}</small></div>
      <nav class="nav" aria-label="Prototype navigation">${nav}</nav>
      <div class="actions">${actions}</div>
    </header>
    ${body}
  </div>`;
}

function metrics(items) {
  return `<div class="metric-grid">${items.map((item) => `<div class="metric"><span>${item[0]}</span><b>${item[1]}</b><code>${item[2]}</code></div>`).join('')}</div>`;
}

function taskRail() {
  return `<aside class="rail">
    <div class="section stack">
      <h2>Task contract</h2>
      ${tag('info', 'suite')}
      <p class="fine">Goal: synthesize specialist review findings into one restrained review packet.</p>
    </div>
    <div class="section stack">
      <h2>Family</h2>
      ${tag('good', 'general')}
      ${tag('warn', 'correctness')}
      ${tag('warn', 'security')}
      ${tag('info', 'verification')}
      ${tag('info', 'simplification')}
      ${tag('info', 'product')}
    </div>
    <div class="section stack">
      <h2>Evidence roots</h2>
      <code>specs/pr-review-suite/taskspec.toml</code>
      <code>docs/review-swarm-vertical-slice.md</code>
      <code>${product.contract}</code>
    </div>
  </aside>`;
}

function inspector(kind = 'candidate') {
  return `<aside class="inspector">
    <div class="section stack">
      <h2>${kind === 'gate' ? 'Launch boundary' : 'Evidence inspector'}</h2>
      ${tag('stop', 'G3 unsigned')}
      ${tag('good', 'sandbox')}
    </div>
    <div class="section">
      <div class="plate">
        <b>composition</b><span><code>${product.composition}</code></span>
        <b>arena</b><span><code>${product.arena}</code></span>
        <b>trace</b><span><code>trace.otel.json</code></span>
        <b>plane</b><span>Bitter Blossom dry-run import</span>
      </div>
    </div>
    <div class="section stack">
      <h2>Residual risk</h2>
      <p class="fine">Specialist members are uneven. Control planes keep posting authority and write authority locked.</p>
    </div>
  </aside>`;
}

function baseline() {
  return shell('Dossier', `<main class="layout catalog-grid">
    <aside class="rail">
      <div class="section stack">
        <h1>Catalog baseline</h1>
        <p class="fine">A long survey of the strongest earlier concepts. Useful for breadth, weaker for repeated operation.</p>
        ${tag('info', 'baseline')}
      </div>
      <div class="section stack">
        <h2>Strongest ingredients</h2>
        ${tag('good', 'operator console')}
        ${tag('good', 'trace lab')}
        ${tag('good', 'gate room')}
        ${tag('good', 'matrix')}
      </div>
    </aside>
    <section class="stage">
      <div class="catalog-tiles">
        <div class="panel stack"><h2>Task model</h2><p>The task is the root object. Benchmarks, hypotheses, compositions, runs, traces, and gates hang from it.</p></div>
        <div class="panel stack"><h2>Dossier</h2><p>Best for orientation and source-of-truth inspection before spend.</p></div>
        <div class="panel stack"><h2>Hypotheses</h2><p>Best for deciding what to mutate next and what result would count.</p></div>
        <div class="panel stack"><h2>Trace lab</h2><p>Best for reading why a candidate scored what it scored.</p></div>
        <div class="panel stack"><h2>Gate room</h2><p>Best for keeping lab evidence separate from deployment trust.</p></div>
        <div class="panel stack"><h2>Matrix</h2><p>Best for selecting the next comparative run from task-level deltas.</p></div>
      </div>
    </section>
  </main>`, '<button class="primary">Open current catalog</button><button>Compare options</button>');
}

function dossier() {
  return shell('Dossier', `<main class="layout" style="grid-template-columns: 280px minmax(0, 1fr) 340px">
    ${taskRail()}
    <section class="stage">
      <div class="section stack">
        <h1>Task dossier</h1>
        <p>The operator starts from one task contract, then drills into benchmark trust, candidate packages, and launch state without leaving the task frame.</p>
        ${metrics([
          ['mode', 'threshold-then-cheap', 'taskspec.toml'],
          ['budget', '$8.00 search cap', 'cost matters'],
          ['gate', 'G2 accepted', 'G3 locked'],
          ['delivery', 'sandbox packet', 'plane-handoff.md'],
        ])}
      </div>
      <div class="section split">
        <div class="panel stack">
          <h2>Task scope</h2>
          <p>PR open event creates a review suite. Specialists find defects. Master reviewer synthesizes one final review.</p>
          ${tag('good', 'read-only')}
        </div>
        <div class="panel stack">
          <h2>Benchmark hypothesis</h2>
          <p>Deterministic fixtures catch correctness, security, and synthesis defects. Judge scoring remains diagnostic until calibrated.</p>
          ${tag('warn', 'eval trust visible')}
        </div>
      </div>
      <div class="section wide-scroll">
        <table class="table">
          <thead><tr><th>Object</th><th>Path</th><th>State</th><th>Action</th></tr></thead>
          <tbody>
            <tr><td>Spec</td><td><code>specs/pr-review-suite/taskspec.toml</code></td><td>${tag('good', 'G1')}</td><td>Inspect contract</td></tr>
            <tr><td>Master arena</td><td><code>arenas/pr-review-master-v0</code></td><td>${tag('warn', 'G2 pending')}</td><td>Read freeze packet</td></tr>
            <tr><td>General agent</td><td><code>deliveries/pr-review</code></td><td>${tag('good', 'delivered')}</td><td>Open handoff</td></tr>
            <tr><td>Sandbox export</td><td><code>launch-dry-run/bitter-blossom.import-packet.toml</code></td><td>${tag('stop', 'G3 locked')}</td><td>Dry run only</td></tr>
          </tbody>
        </table>
      </div>
    </section>
    ${inspector()}
  </main>`);
}

function matrix() {
  return shell('Matrix', `<main class="layout" style="grid-template-columns: minmax(0, 1fr) 340px">
    <section class="stage">
      <div class="section stack">
        <h1>Experiment matrix</h1>
        <p>Candidate selection happens in a dense grid where every aggregate score can open the task-level evidence that produced it.</p>
        ${metrics([
          ['leader', 'Qwen skeptic', '0.8000 mean'],
          ['certified', 'GLM child', 'holdout packet'],
          ['cheapest', 'DeepSeek', '$0.0052 trial'],
          ['risk', 'live-lock missed', 'hard task'],
        ])}
      </div>
      <div class="section wide-scroll">
        <table class="table matrix-table">
          <thead><tr><th>candidate</th><th>export</th><th>formatted</th><th>crash</th><th>live-lock</th><th>measure</th><th>clean</th><th>plugin</th><th>progress</th></tr></thead>
          <tbody>
            <tr><td><b>seed3-qwen3.7-skeptic</b></td><td class="score good">1.00</td><td class="score good">1.00</td><td class="score good">1.00</td><td class="score stop">0.00</td><td class="score stop">0.00</td><td class="score good">1.00</td><td class="score good">1.00</td><td class="score good">1.00</td></tr>
            <tr><td><b>seed4-gpt-5-mini-checklist</b></td><td class="score warn">0.50</td><td class="score good">1.00</td><td class="score stop">0.00</td><td class="score stop">0.00</td><td class="score good">0.80</td><td class="score good">1.00</td><td class="score good">1.00</td><td class="score good">1.00</td></tr>
            <tr><td><b>seed1-deepseek-v4</b></td><td class="score warn">0.50</td><td class="score stop">0.00</td><td class="score stop">0.00</td><td class="score stop">0.00</td><td class="score good">1.00</td><td class="score good">1.00</td><td class="score good">1.00</td><td class="score good">1.00</td></tr>
            <tr><td><b>g1a-qwen-on-glm</b></td><td class="score warn">0.50</td><td class="score good">1.00</td><td class="score warn">0.50</td><td class="score stop">0.00</td><td class="score good">1.00</td><td class="score warn">0.50</td><td class="score warn">0.50</td><td class="score warn">0.50</td></tr>
          </tbody>
        </table>
      </div>
    </section>
    <aside class="inspector">
      <div class="section stack">
        <h2>Candidate read</h2>
        ${tag('warn', 'choose next')}
        <p>Qwen covers more tasks, but certification quality regressed from the GLM child. Run a targeted live-lock mutation next.</p>
      </div>
      <div class="section">
        <div class="plate">
          <b>quality</b><span>Qwen at 0.8000 mean</span>
          <b>certified</b><span>GLM child only</span>
          <b>cost leader</b><span>DeepSeek at $0.0052 per trial</span>
          <b>next run</b><span>live-lock specialist mutation</span>
        </div>
      </div>
    </aside>
  </main>`, '<button class="primary">Compare packet</button><button>Certify selected</button>');
}

function trace() {
  return shell('Trace', `<main class="layout" style="grid-template-columns: 320px minmax(0, 1fr) 340px">
    <aside class="rail">
      <div class="section stack">
        <h1>Trace drilldown</h1>
        <p class="fine">Aggregate score opens a reproducible path through run, trial, transcript, finding, and scorer result.</p>
      </div>
      <div class="section stack">
        ${tag('stop', 'missed defect')}
        <code>py-export-clear</code>
        <code>trial 3</code>
        <code>artifact pointer retained</code>
      </div>
    </aside>
    <section class="stage">
      <div class="section">
        <div class="timeline">
          <div class="when">00:00.000</div><div><b>workspace copied</b><p class="fine">Candidate-visible environment mounted without tests or solution.</p></div>
          <div class="when">00:02.431</div><div><b>prompt packet resolved</b><p class="fine">composition hash ${product.composition}</p></div>
          <div class="when">00:41.018</div><div><b>findings emitted</b><p class="fine">Two findings, no malformed output, cost recorded as null if unavailable.</p></div>
          <div class="when">00:41.114</div><div><b>scorer matched</b><p class="fine">Matched export defect. Missed live-lock defect. False positives zero.</p></div>
          <div class="when">00:41.130</div><div><b>trial committed</b><p class="fine">JSONL record survives. Heavy transcript stays local with artifact index.</p></div>
        </div>
      </div>
      <div class="section split">
        <div class="panel stack"><h2>Candidate output</h2><code>{"file":"crates/daedalus-core/src/export.rs","category":"approval-gate"}</code><p class="fine">Actionable, correct category, line in expected range.</p></div>
        <div class="panel stack"><h2>Scorer result</h2><code>reward=0.50 recall=0.50 fp=0</code><p class="fine">The missing live-lock path is the mutation seed.</p></div>
      </div>
    </section>
    ${inspector()}
  </main>`, '<button class="primary">Open artifact</button><button>Promote mutation</button>');
}

function swarm() {
  return shell('Swarm', `<main class="layout" style="grid-template-columns: 280px minmax(0, 1fr) 320px">
    ${taskRail()}
    <section class="stage">
      <div class="section stack">
        <h1>Review swarm map</h1>
        <p>The suite is not one reviewer. It is a measured review organization with member tasks and a master synthesis task.</p>
      </div>
      <div class="section swarm-grid">
        <div class="agent"><h2>General</h2>${tag('good', 'certified')}<p class="fine">Broad PR review baseline. Delivery exists.</p></div>
        <div class="agent"><h2>Correctness</h2>${tag('warn', 'weak baseline')}<p class="fine">Needs live-lock and clean-trap improvement.</p></div>
        <div class="agent"><h2>Security</h2>${tag('warn', 'unstable')}<p class="fine">Injection behavior requires another arena pass.</p></div>
        <div class="agent"><h2>Verification</h2>${tag('info', 'scaffold')}<p class="fine">Gate, CI, and evidence-quality review.</p></div>
        <div class="agent master"><h2>Master</h2>${tag('warn', 'G2 pending')}<p class="fine">Synthesizes one restrained review from member outputs.</p></div>
      </div>
      <div class="section split">
        <div class="panel stack"><h2>Orchestration rule</h2><p>Specialists run in parallel. Master reads only member findings, evidence, and residual risk.</p></div>
        <div class="panel stack"><h2>Plane rule</h2><p>Bitter Blossom and Olympus own triggers, posting authority, rollback, and operator-visible state.</p></div>
      </div>
    </section>
    <aside class="inspector">
      <div class="section stack">
        <h2>Swarm readiness</h2>
        ${tag('stop', 'sandbox only')}
        <p class="fine">The UI keeps sandbox delivery visually separate from primary reviewer deployment.</p>
      </div>
      <div class="section">
        <table class="table">
          <tbody>
            <tr><th>posting</th><td>control plane</td></tr>
            <tr><th>dedupe</th><td>control plane</td></tr>
            <tr><th>write auth</th><td>G4 required</td></tr>
            <tr><th>trace ingest</th><td>G5 required</td></tr>
          </tbody>
        </table>
      </div>
    </aside>
  </main>`, '<button class="primary">Open suite</button><button>Run member</button>');
}

function gateRoom() {
  return shell('Gate', `<main class="layout" style="grid-template-columns: 260px minmax(0, 1fr) 360px">
    <aside class="rail">
      <div class="section stack"><h2>G1 spec</h2>${tag('good', 'signed')}<p class="fine">Spend approval exists.</p></div>
      <div class="section stack"><h2>G2 eval</h2>${tag('warn', 'member caveats')}<p class="fine">Internal benchmark only.</p></div>
      <div class="section stack"><h2>G3 launch</h2>${tag('stop', 'unsigned')}<p class="fine">No primary reviewer deployment.</p></div>
      <div class="section stack"><h2>G4 write</h2>${tag('stop', 'locked')}<p class="fine">No write authority.</p></div>
      <div class="section stack"><h2>G5 ingest</h2>${tag('stop', 'locked')}<p class="fine">No production-data re-ingestion.</p></div>
    </aside>
    <section class="stage">
      <div class="section stack">
        <h1>Contract gate room</h1>
        <p>Launch is not a button at the end of the lab. It is a contract screen that binds evidence, permissions, and plane handoff.</p>
        ${metrics([
          ['contract', 'contract.v1', product.contract],
          ['persona', 'byte-identical prompt', 'persona.md'],
          ['handoff', 'human-reviewable', 'plane-handoff.md'],
          ['launch pack', 'dry-run only', 'G3 locked'],
        ])}
      </div>
      <div class="section">
        <div class="plate">
          <b>trigger</b><span>GitHub PR open event, sandbox reviewer</span>
          <b>output</b><span>Structured findings, evidence, confidence, residual risk</span>
          <b>permissions</b><span>read-only workspace, env allowlist, no writes</span>
          <b>observability</b><span>JSONL records plus derived trace export</span>
          <b>escalation</b><span>human review before posting or promotion</span>
        </div>
      </div>
    </section>
    ${inspector('gate')}
  </main>`, '<button class="primary">Dry-run packet</button><button>Request G3</button>');
}

function foundry() {
  return shell('Hypotheses', `<main class="layout" style="grid-template-columns: 320px minmax(0, 1fr) 340px">
    <aside class="rail">
      <div class="section stack">
        <h1>Hypothesis foundry</h1>
        <p class="fine">The autoresearch loop is controlled by explicit bets, not vibes.</p>
      </div>
      <div class="section stack">
        ${tag('good', 'headroom check')}
        ${tag('warn', 'probe noise')}
        ${tag('info', 'budget visible')}
      </div>
    </aside>
    <section class="stage">
      <div class="section stack">
        <h2>Current loop</h2>
        <div class="mutation"><b>1</b><span><b>live-lock mutation</b><br><span class="fine">Add caller-path search before final finding emission.</span></span>${tag('warn', 'queued')}</div>
        <div class="mutation"><b>2</b><span><b>clean-trap restraint</b><br><span class="fine">Measure invented findings on no-defect tasks.</span></span>${tag('info', 'seed')}</div>
        <div class="mutation"><b>3</b><span><b>cost envelope</b><br><span class="fine">Try cheaper model after quality threshold is met.</span></span>${tag('good', 'ready')}</div>
      </div>
      <div class="section triple">
        <div class="panel stack"><h2>Hypothesis</h2><p>A skeptic prompt improves high-risk correctness recall without raising false positives.</p></div>
        <div class="panel stack"><h2>Oracle</h2><p>Reward rises on live-lock and crash tasks, while clean task remains silent.</p></div>
        <div class="panel stack"><h2>Stop rule</h2><p>After two failed mutations, debrief arena quality, tactic choice, and task tractability.</p></div>
      </div>
    </section>
    <aside class="inspector">
      <div class="section stack">
        <h2>Debrief packet</h2>
        <p class="fine">Every failed branch records what was tried and why the loop stopped.</p>
      </div>
      <div class="section">
        <div class="plate">
          <b>bad eval</b><span>fixture cannot distinguish candidates</span>
          <b>bad tactic</b><span>mutation does not target failure mode</span>
          <b>bad model</b><span>provider underperforms task shape</span>
          <b>intractable</b><span>score plateau after adversarial review</span>
        </div>
      </div>
    </aside>
  </main>`, '<button class="primary">Start run</button><button>Write debrief</button>');
}

const SPECS = {
  'DW-0': baseline,
  'DW-1': dossier,
  'DW-2': matrix,
  'DW-3': trace,
  'DW-4': swarm,
  'DW-5': gateRoom,
  'DW-6': foundry,
};

function render() {
  const id = location.hash.replace('#', '') || 'DW-0';
  const builder = SPECS[id] || SPECS['DW-0'];
  document.querySelector('#mount').innerHTML = builder();
  document.querySelectorAll('a[href="#0"]').forEach((link) => {
    link.addEventListener('click', (event) => event.preventDefault());
  });
}

window.addEventListener('hashchange', render);
render();
