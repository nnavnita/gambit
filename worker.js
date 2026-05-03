import init, { Engine } from './pkg/gambit.js';

let engine;

init().then(() => {
  engine = new Engine();
  postMessage({ type: 'ready' });
}).catch(err => {
  console.error('WASM init failed:', err);
  postMessage({ type: 'init_error', message: String(err) });
});

onmessage = (e) => {
  try {
    const { fen, depth } = e.data;
    const move = engine.best_move(fen, depth);
    postMessage({ type: 'move', move });
  } catch (err) {
    console.error('Engine error:', err);
    postMessage({ type: 'move', move: '' });
  }
};
