import init, { Engine } from './pkg/gambit.js';

let engine;

init().then(() => {
  engine = new Engine();
  postMessage({ type: 'ready' });
}).catch(err => {
  console.error('Eval worker init failed:', err);
});

onmessage = (e) => {
  try {
    const { fen, depth } = e.data;
    const score    = engine.eval_position(fen, depth);
    const bestMove = engine.best_move(fen, depth);
    postMessage({ type: 'eval', score, bestMove });
  } catch (err) {
    console.error('Eval worker error:', err);
    postMessage({ type: 'eval', score: 0, bestMove: '' });
  }
};
