import init, { Engine } from './pkg/gambit.js';

let engine;
let ready = false;
const pending = []; // queue messages until engine is ready

init().then(() => {
  engine = new Engine();
  engine.set_info_callback((depth, cp, pv, nodes, ms) => {
    postMessage({ type: 'info', depth, cp, pv, nodes, ms });
  });
  ready = true;
  postMessage({ type: 'ready' });
  // drain anything that arrived before init resolved
  while (pending.length) handle(pending.shift());
}).catch(err => {
  console.error('WASM init failed:', err);
  postMessage({ type: 'init_error', message: String(err) });
});

onmessage = (e) => {
  if (!ready) { pending.push(e.data); return; }
  handle(e.data);
};

function fail(type, err) {
  console.error('Engine error (' + type + '):', err);
  if (type === 'eval') postMessage({ type: 'eval', score: 0 });
  else if (type === 'hint') postMessage({ type: 'hint', move: '' });
  else if (type === 'multi') postMessage({ type: 'multi', csv: '' });
  else postMessage({ type: 'move', move: '' });
}

function handle(msg) {
  const m = msg || {};
  const type = m.type;
  try {
    if (type === 'set_history') {
      engine.set_history(typeof m.fens === 'string' ? m.fens : '');
      return;
    }
    // Every remaining branch needs a FEN; bail early with a useful log.
    if (typeof m.fen !== 'string' || m.fen.length === 0) {
      console.warn('worker: missing fen in message', m);
      fail(type, new Error('missing fen'));
      return;
    }
    if (type === 'eval') {
      const score = engine.eval_position(m.fen, m.depth || 4);
      postMessage({ type: 'eval', score });
      return;
    }
    if (type === 'hint') {
      const move = engine.best_move(m.fen, m.depth || 4);
      postMessage({ type: 'hint', move });
      return;
    }
    if (type === 'multi') {
      const csv = engine.analyse_multi(m.fen, m.depth || 4, m.n || 3);
      postMessage({ type: 'multi', csv });
      return;
    }
    if (type === 'move_time') {
      const move = engine.best_move_time(m.fen, m.ms || 1000);
      postMessage({ type: 'move', move });
      return;
    }
    // default: depth-bounded best move
    const depth = Number.isFinite(m.depth) && m.depth > 0 ? (m.depth | 0) : 3;
    const move = engine.best_move(m.fen, depth);
    postMessage({ type: 'move', move });
  } catch (err) {
    fail(type, err);
  }
}
