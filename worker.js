import init, { Engine } from './pkg/gambit.js';

let engine;

init().then(() => {
  engine = new Engine();
  // Stream depth-by-depth info to the main thread.
  engine.set_info_callback((depth, cp, pv, nodes, ms) => {
    postMessage({ type: 'info', depth, cp, pv, nodes, ms });
  });
  postMessage({ type: 'ready' });
}).catch(err => {
  console.error('WASM init failed:', err);
  postMessage({ type: 'init_error', message: String(err) });
});

onmessage = (e) => {
  const msg = e.data;
  const { type } = msg;
  try {
    if (type === 'set_history') {
      engine.set_history(msg.fens || '');
      return;
    }
    if (type === 'eval') {
      const score = engine.eval_position(msg.fen, msg.depth || 4);
      postMessage({ type: 'eval', score });
      return;
    }
    if (type === 'hint') {
      const move = engine.best_move(msg.fen, msg.depth || 4);
      postMessage({ type: 'hint', move });
      return;
    }
    if (type === 'multi') {
      const csv = engine.analyse_multi(msg.fen, msg.depth || 4, msg.n || 3);
      postMessage({ type: 'multi', csv });
      return;
    }
    if (type === 'move_time') {
      const move = engine.best_move_time(msg.fen, msg.ms || 1000);
      postMessage({ type: 'move', move });
      return;
    }
    // default: depth-bounded best move
    const move = engine.best_move(msg.fen, msg.depth);
    postMessage({ type: 'move', move });
  } catch (err) {
    console.error('Engine error:', err);
    if (type === 'eval') postMessage({ type: 'eval', score: 0 });
    else if (type === 'hint') postMessage({ type: 'hint', move: '' });
    else if (type === 'multi') postMessage({ type: 'multi', csv: '' });
    else postMessage({ type: 'move', move: '' });
  }
};
