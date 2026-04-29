import init, { Engine } from './pkg/gambit.js';
import { Chess } from 'https://esm.sh/chess.js@1';

let engine;
(async () => {
  await init();
  engine = new Engine();
  self.postMessage({ type: 'ready' });
})();

self.onmessage = (e) => {
  if (e.data.type === 'generate') {
    try {
      const puzzle = generatePuzzle(e.data.seed);
      self.postMessage({ type: 'puzzle', puzzle });
    } catch (err) {
      self.postMessage({ type: 'error', message: err.message });
    }
  }
};

// ── Seeded RNG (xorshift32) ──────────────────────────────────────────────────
function makeRng(seed) {
  let s = (seed >>> 0) || 0xDEADBEEF;
  return () => {
    s ^= s << 13;
    s ^= s >> 17;
    s ^= s << 5;
    return (s >>> 0) / 4294967296;
  };
}

// ── Puzzle generation ────────────────────────────────────────────────────────
function generatePuzzle(seed) {
  const rng = makeRng(seed);

  // Play engine-vs-engine game with seeded randomness
  const chess = new Chess();
  const candidates = []; // positions to check for tactics

  for (let moveNo = 0; moveNo < 45; moveNo++) {
    if (chess.isGameOver()) break;
    const fen = chess.fen();
    const legalMoves = chess.moves({ verbose: true });
    if (!legalMoves.length) break;

    // Randomness: high early (varied openings), low later (realistic play)
    const randomChance = moveNo < 6 ? 0.5 : moveNo < 12 ? 0.2 : 0.1;
    const depth = moveNo < 8 ? 2 : 3;

    let chosenUci;
    if (rng() < randomChance) {
      const m = legalMoves[Math.floor(rng() * legalMoves.length)];
      chosenUci = m.from + m.to + (m.promotion || '');
    } else {
      chosenUci = engine.best_move(fen, depth);
    }
    if (!chosenUci) break;

    // Record candidate positions after the opening
    if (moveNo >= 8 && moveNo <= 35) {
      candidates.push(fen);
    }

    try {
      chess.move({ from: chosenUci.slice(0,2), to: chosenUci.slice(2,4), promotion: chosenUci[4] || 'q' });
    } catch(e) { break; }
  }

  // ── Scan candidates for tactical shots ──────────────────────────────────────
  let bestPuzzle = null;
  let bestPuzzleScore = -Infinity;

  for (const fen of candidates) {
    const c = new Chess();
    c.load(fen);
    if (c.isGameOver()) continue;

    const evalBefore = engine.eval_position(fen, 3); // side-to-move perspective
    if (Math.abs(evalBefore) > 600) continue; // skip already-decided positions

    const bestUci = engine.best_move(fen, 4);
    if (!bestUci || bestUci.length < 4) continue;

    // Apply best move
    const cc = new Chess();
    cc.load(fen);
    let moveObj;
    try {
      moveObj = cc.move({ from: bestUci.slice(0,2), to: bestUci.slice(2,4), promotion: bestUci[4] || 'q' });
    } catch(e) { continue; }
    if (!moveObj) continue;

    // Eval after (negated — now opponent's turn)
    const evalAfter = engine.eval_position(cc.fen(), 3);
    const gain = -evalAfter - evalBefore; // how much the best move improved our position

    // Good puzzle: clear tactical gain, near-equal before
    if (gain < 100) continue;

    const puzzleScore = gain - Math.abs(evalBefore) * 0.3;
    if (puzzleScore > bestPuzzleScore) {
      bestPuzzleScore = puzzleScore;
      bestPuzzle = {
        fen,
        solution: bestUci,
        solutionSan: moveObj.san,
        turn: c.turn(),
        gain: Math.round(gain),
        isCapture: !!moveObj.captured,
        isCheck: cc.inCheck(),
        isCheckmate: cc.isCheckmate(),
      };
    }
  }

  // Fallback: just return any valid position with a best move
  if (!bestPuzzle) {
    for (const fen of candidates) {
      const c = new Chess();
      c.load(fen);
      if (c.isGameOver()) continue;
      const bestUci = engine.best_move(fen, 3);
      if (!bestUci || bestUci.length < 4) continue;
      const cc = new Chess();
      cc.load(fen);
      let moveObj;
      try { moveObj = cc.move({ from: bestUci.slice(0,2), to: bestUci.slice(2,4), promotion: bestUci[4] || 'q' }); } catch(e) { continue; }
      if (moveObj) {
        bestPuzzle = { fen, solution: bestUci, solutionSan: moveObj.san, turn: c.turn(), gain: 0, isCapture: !!moveObj.captured, isCheck: cc.inCheck(), isCheckmate: cc.isCheckmate() };
        break;
      }
    }
  }

  return bestPuzzle;
}
