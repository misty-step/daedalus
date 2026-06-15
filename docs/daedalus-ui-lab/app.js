const OPTIONS = [
  {
    id: 'DW-0',
    label: 'Catalog baseline',
    note: 'Long-scroll survey of the current visual catalog.',
  },
  {
    id: 'DW-1',
    label: 'Task dossier',
    note: 'One task contract as the root object with evidence beside it.',
  },
  {
    id: 'DW-2',
    label: 'Experiment matrix',
    note: 'Candidate comparison is the primary surface.',
  },
  {
    id: 'DW-3',
    label: 'Trace drilldown',
    note: 'From aggregate score to trial, transcript, and artifact.',
  },
  {
    id: 'DW-4',
    label: 'Swarm map',
    note: 'Suite task with specialists and master reviewer synthesis.',
  },
  {
    id: 'DW-5',
    label: 'Gate room',
    note: 'Approvals and plane handoff are first-class launch objects.',
  },
  {
    id: 'DW-6',
    label: 'Hypothesis foundry',
    note: 'Research loop control for mutations and debriefs.',
  },
];

const state = {
  active: localStorage.getItem('daedalus-ui-lab-active') || 'DW-0',
  size: localStorage.getItem('daedalus-ui-lab-size') || 'fit',
  customW: Number(localStorage.getItem('daedalus-ui-lab-custom-w') || 1440),
  customH: Number(localStorage.getItem('daedalus-ui-lab-custom-h') || 900),
};

const optionList = document.querySelector('#optionList');
const frame = document.querySelector('#prototypeFrame');
const frameBox = document.querySelector('#previewFrame');
const readout = document.querySelector('#scaleReadout');
const customW = document.querySelector('#customW');
const customH = document.querySelector('#customH');

customW.value = state.customW;
customH.value = state.customH;

function renderOptions() {
  optionList.innerHTML = '';
  for (const option of OPTIONS) {
    const button = document.createElement('button');
    button.className = 'option-button';
    button.type = 'button';
    button.setAttribute('aria-current', option.id === state.active ? 'true' : 'false');
    button.dataset.id = option.id;
    button.innerHTML = `<b>${option.id} ${option.label}</b><small>${option.note}</small>`;
    button.addEventListener('click', () => selectOption(option.id));
    optionList.appendChild(button);
  }
}

function selectOption(id) {
  state.active = id;
  localStorage.setItem('daedalus-ui-lab-active', id);
  const nextSrc = `frame.html?v=1#${id}`;
  if (!frame.src.endsWith(nextSrc)) {
    frame.src = nextSrc;
  }
  renderOptions();
}

function selectedSize() {
  if (state.size === 'fit') {
    const area = document.querySelector('.preview-area').getBoundingClientRect();
    return { width: Math.max(320, Math.floor(area.width - 36)), height: Math.max(520, Math.floor(area.height - 36)), label: 'fit' };
  }
  if (state.size === 'custom') {
    return { width: state.customW, height: state.customH, label: `${state.customW}x${state.customH}` };
  }
  const [width, height] = state.size.split('x').map(Number);
  return { width, height, label: state.size };
}

function applyViewport() {
  const { width, height, label } = selectedSize();
  const area = document.querySelector('.preview-area').getBoundingClientRect();
  const scale = Math.min(1, (area.width - 36) / width, (area.height - 36) / height);
  frame.style.width = `${width}px`;
  frame.style.height = `${height}px`;
  frameBox.style.width = `${width}px`;
  frameBox.style.height = `${height}px`;
  frameBox.style.transform = `translate(-50%, -50%) scale(${scale})`;
  readout.textContent = `${label} - ${width}x${height} - ${Math.round(scale * 100)}%`;
  document.querySelectorAll('[data-size]').forEach((button) => {
    button.classList.toggle('is-active', button.dataset.size === state.size);
  });
}

document.querySelectorAll('[data-size]').forEach((button) => {
  button.addEventListener('click', () => {
    state.size = button.dataset.size;
    localStorage.setItem('daedalus-ui-lab-size', state.size);
    applyViewport();
  });
});

document.querySelector('#applyCustom').addEventListener('click', () => {
  state.customW = Math.max(320, Number(customW.value) || 1440);
  state.customH = Math.max(480, Number(customH.value) || 900);
  state.size = 'custom';
  localStorage.setItem('daedalus-ui-lab-custom-w', state.customW);
  localStorage.setItem('daedalus-ui-lab-custom-h', state.customH);
  localStorage.setItem('daedalus-ui-lab-size', state.size);
  applyViewport();
});

window.addEventListener('resize', applyViewport);
window.addEventListener('keydown', (event) => {
  const index = OPTIONS.findIndex((option) => option.id === state.active);
  if (event.key === 'ArrowDown' || event.key === 'ArrowRight') {
    event.preventDefault();
    selectOption(OPTIONS[Math.min(OPTIONS.length - 1, index + 1)].id);
  }
  if (event.key === 'ArrowUp' || event.key === 'ArrowLeft') {
    event.preventDefault();
    selectOption(OPTIONS[Math.max(0, index - 1)].id);
  }
});

renderOptions();
selectOption(state.active);
applyViewport();
