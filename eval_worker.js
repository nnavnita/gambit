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
    // Get best move string, then evaluate resulting position's score
    // We use the engine's internal eval by searching and reading the score
    // from the TT — approximated by negating the eval of the position
    // after the best move. Simpler: evaluate by searching depth 1 deeper
    // and extracting the score directly via a dedicated export.
    const score = engine.eval_position(fen, depth);
    postMessage({ type: 'eval', score });
  } catch (err) {
    console.error('Eval worker error:', err);
    postMessage({ type: 'eval', score: 0 });
  }
};
