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

// ── Seeded RNG ────────────────────────────────────────────────────────────────
function makeRng(seed) {
  let s = (seed >>> 0) || 0xDEADBEEF;
  return () => {
    s ^= s << 13; s ^= s >> 17; s ^= s << 5;
    return (s >>> 0) / 4294967296;
  };
}

// ── Main generation ───────────────────────────────────────────────────────────
function generatePuzzle(seed) {
  const rng = makeRng(seed);

  // Play engine-vs-engine game with seeded randomness
  const chess = new Chess();
  const candidates = [];

  for (let moveNo = 0; moveNo < 45; moveNo++) {
    if (chess.isGameOver()) break;
    const fen = chess.fen();
    const legalMoves = chess.moves({ verbose: true });
    if (!legalMoves.length) break;

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

    if (moveNo >= 8 && moveNo <= 35) candidates.push(fen);

    try {
      chess.move({ from: chosenUci.slice(0,2), to: chosenUci.slice(2,4), promotion: chosenUci[4] || 'q' });
    } catch(e) { break; }
  }

  // ── Find best puzzle candidate ────────────────────────────────────────────
  let bestPuzzle = null;
  let bestScore = -Infinity;

  for (const fen of candidates) {
    const c = new Chess();
    c.load(fen);
    if (c.isGameOver()) continue;

    const evalBefore = engine.eval_position(fen, 3);
    if (Math.abs(evalBefore) > 600) continue; // already decided

    const bestUci = engine.best_move(fen, 4);
    if (!bestUci || bestUci.length < 4) continue;

    const cc = new Chess();
    cc.load(fen);
    let moveObj;
    try { moveObj = cc.move({ from: bestUci.slice(0,2), to: bestUci.slice(2,4), promotion: bestUci[4] || 'q' }); }
    catch(e) { continue; }
    if (!moveObj) continue;

    const evalAfter = engine.eval_position(cc.fen(), 3);
    const gain = -evalAfter - evalBefore;
    if (gain < 100) continue;

    const score = gain - Math.abs(evalBefore) * 0.3;
    if (score > bestScore) {
      bestScore = score;
      bestPuzzle = { fen, firstMove: bestUci, turn: c.turn(), gain: Math.round(gain) };
    }
  }

  if (!bestPuzzle) {
    // Fallback: any position with a clear best move
    for (const fen of candidates) {
      const c = new Chess();
      c.load(fen);
      if (c.isGameOver()) continue;
      const bestUci = engine.best_move(fen, 3);
      if (bestUci && bestUci.length >= 4) {
        bestPuzzle = { fen, firstMove: bestUci, turn: c.turn(), gain: 0 };
        break;
      }
    }
  }

  if (!bestPuzzle) return null;

  // ── Build full solution line ──────────────────────────────────────────────
  const line = buildLine(bestPuzzle.fen, bestPuzzle.turn);
  if (!line.length) return null;

  const lastPlayerMove = line.filter(m => m.isPlayer).pop();
  const cc = new Chess();
  cc.load(bestPuzzle.fen);
  for (const m of line) {
    try { cc.move({ from: m.uci.slice(0,2), to: m.uci.slice(2,4), promotion: m.uci[4] || 'q' }); } catch(e) {}
  }

  return {
    fen: bestPuzzle.fen,
    turn: bestPuzzle.turn,
    gain: bestPuzzle.gain,
    line,
    isCheckmate: line[line.length - 1]?.isCheckmate || false,
  };
}

// ── Build multi-move solution line ────────────────────────────────────────────
// Returns array of { uci, san, isPlayer, isCheckmate }
// Alternates: player move → engine response → player move → ...
// Ends on a player move once position is clearly won.
function buildLine(startFen, playerColor) {
  const line = [];
  let fen = startFen;
  let playerMoveCount = 0;

  for (let half = 0; half < 10; half++) {
    const c = new Chess();
    c.load(fen);
    if (c.isGameOver()) break;

    const isPlayer = c.turn() === playerColor;
    const depth = isPlayer ? 5 : 4;
    const bestUci = engine.best_move(fen, depth);
    if (!bestUci || bestUci.length < 4) break;

    let moveObj;
    try {
      moveObj = c.move({ from: bestUci.slice(0,2), to: bestUci.slice(2,4), promotion: bestUci[4] || 'q' });
    } catch(e) { break; }
    if (!moveObj) break;

    const isCheckmate = c.isCheckmate();
    line.push({ uci: bestUci, san: moveObj.san, isPlayer, isCheckmate });
    fen = c.fen();

    if (isPlayer) {
      playerMoveCount++;

      // Puzzle ends here if player delivers checkmate
      if (isCheckmate) break;

      // Eval from opponent's POV after player's move — negative means opponent is bad
      const evalAfter = engine.eval_position(fen, 3);
      const playerAdv = -evalAfter; // positive = good for player

      // After ≥2 player moves, stop if clearly winning (>400cp)
      if (playerMoveCount >= 2 && playerAdv > 400) {
        // Include one engine "damage control" response then stop
        const ec = new Chess();
        ec.load(fen);
        if (!ec.isGameOver()) {
          const engUci = engine.best_move(fen, 3);
          if (engUci && engUci.length >= 4) {
            let engMove;
            try { engMove = ec.move({ from: engUci.slice(0,2), to: engUci.slice(2,4), promotion: engUci[4] || 'q' }); } catch(e) {}
            if (engMove) {
              line.push({ uci: engUci, san: engMove.san, isPlayer: false, isCheckmate: false });
              fen = ec.fen();
            }
          }
          // Now one final winning player move
          const lastUci = engine.best_move(fen, 5);
          if (lastUci && lastUci.length >= 4) {
            const fc = new Chess();
            fc.load(fen);
            let lastMove;
            try { lastMove = fc.move({ from: lastUci.slice(0,2), to: lastUci.slice(2,4), promotion: lastUci[4] || 'q' }); } catch(e) {}
            if (lastMove) {
              line.push({ uci: lastUci, san: lastMove.san, isPlayer: true, isCheckmate: fc.isCheckmate() });
            }
          }
        }
        break;
      }

      // After 1 player move with massive gain (queen capture etc.), just stop
      if (playerMoveCount === 1 && playerAdv > 700) break;

      // Hard cap: 3 player moves max
      if (playerMoveCount >= 3) break;
    }
  }

  // Trim any trailing engine moves — puzzle must end on player move
  while (line.length && !line[line.length - 1].isPlayer) {
    line.pop();
  }

  return line;
}
