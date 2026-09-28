const HABIT_VERSION = 2;
const CONTINUOUS_SELECTION_WINDOW_MS = 30 * 60 * 1000;
const MAX_REWARD_MULTIPLIER = 5;
const MAX_CORRECTION_MULTIPLIER = 5;

export function createEmptyHabitState() {
  return {
    version: HABIT_VERSION,
    apps: {},
    lastSelected: "",
    lastSelectedAt: 0,
    sameSelectionStreak: 0,
    correctionStreak: 0,
  };
}

export function normalizeHabitState(value) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return createEmptyHabitState();
  }

  if (
    value.version === HABIT_VERSION &&
    value.apps &&
    typeof value.apps === "object"
  ) {
    return {
      ...createEmptyHabitState(),
      ...value,
      apps: Object.fromEntries(
        Object.entries(value.apps).map(([name, entry]) => [
          name,
          {
            score: Math.max(0, Number(entry?.score) || 0),
            useCount: Math.max(0, Number(entry?.useCount) || 0),
            lastUsedAt: Math.max(0, Number(entry?.lastUsedAt) || 0),
          },
        ])
      ),
    };
  }

  // Backward compatibility with the old { appName: score } structure.
  const state = createEmptyHabitState();
  for (const [name, score] of Object.entries(value)) {
    if (typeof score === "number" && Number.isFinite(score)) {
      state.apps[name] = {
        score: Math.max(0, score),
        useCount: Math.max(0, score),
        lastUsedAt: 0,
      };
    }
  }
  return state;
}

function highestScoredApp(apps) {
  return Object.entries(apps).reduce((highest, [name, entry]) => {
    if (!highest || entry.score > highest.entry.score) {
      return { name, entry };
    }
    return highest;
  }, null);
}

export function updateHabitState(value, appName, now = Date.now()) {
  const state = normalizeHabitState(value);
  if (!appName) return state;

  const selected = state.apps[appName] || {
    score: 0,
    useCount: 0,
    lastUsedAt: 0,
  };
  const previousHighest = highestScoredApp(state.apps);
  const isContinuousSelection =
    state.lastSelected === appName &&
    now - state.lastSelectedAt <= CONTINUOUS_SELECTION_WINDOW_MS;

  state.sameSelectionStreak = isContinuousSelection
    ? Math.min(state.sameSelectionStreak + 1, MAX_REWARD_MULTIPLIER)
    : 1;

  // If the user deliberately chooses an app other than the current learned winner,
  // progressively lower the stale winner so the ranking can correct itself quickly.
  if (
    previousHighest &&
    previousHighest.name !== appName &&
    previousHighest.entry.score > selected.score
  ) {
    state.correctionStreak = Math.min(
      state.correctionStreak + 1,
      MAX_CORRECTION_MULTIPLIER
    );
    previousHighest.entry.score = Math.max(
      0,
      previousHighest.entry.score - state.correctionStreak
    );
  } else {
    state.correctionStreak = 0;
  }

  selected.score += state.sameSelectionStreak;
  selected.useCount += 1;
  selected.lastUsedAt = now;
  state.apps[appName] = selected;
  state.lastSelected = appName;
  state.lastSelectedAt = now;
  return state;
}

function recencyBonus(lastUsedAt, now) {
  const age = now - lastUsedAt;
  if (!lastUsedAt || age < 0) return 0;
  if (age <= 60 * 60 * 1000) return 2;
  if (age <= 24 * 60 * 60 * 1000) return 1;
  if (age <= 7 * 24 * 60 * 60 * 1000) return 0.5;
  return 0;
}

export function habitRank(stateValue, appName, now = Date.now()) {
  const state =
    stateValue?.version === HABIT_VERSION && stateValue?.apps
      ? stateValue
      : normalizeHabitState(stateValue);
  const app = state.apps[appName];
  if (!app) return 0;
  return app.score + recencyBonus(app.lastUsedAt, now);
}
