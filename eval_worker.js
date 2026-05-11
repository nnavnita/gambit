import init, { Engine } from './pkg/gambit.js';

let engine;

init().then(() => {
  engine = new Engine();
  postMessage({ type: 'ready' });
}).catch(err => {
  console.error('Eval worker init failed:', err);
});

onmessage = (e) => {
  const { fen, depth, type } = e.data;
  try {
    if (type === 'multi') {
      // Top-2 candidate moves with white-perspective cp scores.
      const csv = engine.analyse_multi(fen, depth, 2);
      const items = csv.split(',').filter(Boolean).map(p => {
        const [uci, cp] = p.split(':');
        return { uci, cp: parseInt(cp) };
      });
      const top = items[0] || { uci: '', cp: 0 };
      const second = items[1] || null;
      postMessage({
        type: 'multi',
        score: top.cp,
        bestMove: top.uci,
        secondScore: second ? second.cp : null,
        secondMove: second ? second.uci : null,
      });
      return;
    }
    const score = engine.eval_position(fen, depth);
    const bestMove = engine.best_move(fen, depth);
    postMessage({ type: 'eval', score, bestMove });
  } catch (err) {
    console.error('Eval worker error:', err);
    if (type === 'multi') postMessage({ type: 'multi', score: 0, bestMove: '', secondScore: null, secondMove: null });
    else postMessage({ type: 'eval', score: 0, bestMove: '' });
  }
};
