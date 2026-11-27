/**
 * Mutator categories grouped like Rocket League's private match settings.
 * Keys are Rocket Plugin's internal category names; anything not listed
 * (new categories from a newer config) lands in "special".
 */
export const MUTATOR_GROUPS = [
  {
    key: "match",
    categories: ["GameTimes", "GameScores", "OvertimeRules", "MaxTimeRules", "MatchGames", "FreePlay"],
  },
  {
    key: "ball",
    categories: [
      "BallMaxSpeed",
      "BallType",
      "BallWeight",
      "BallScale",
      "BallBounciness",
      "BallGravity",
      "MultiBall",
    ],
  },
  { key: "boost", categories: ["Boosters", "BoosterStrengths", "BoostRestriction", "Items"] },
  {
    key: "game",
    categories: [
      "GameSpeed",
      "Gravity",
      "Demolish",
      "RespawnTime",
      "CarJumps",
      "DodgeInputTimer",
      "InputRestriction",
    ],
  },
  {
    key: "scoring",
    categories: ["GoalScore", "AerialGoalScore", "AssistScore", "DemolishScore", "GoalScoringRules"],
  },
  {
    key: "special",
    categories: [
      "AudioRules",
      "GameEventRules",
      "TritipModeRules",
      "TerritoryRules",
      "BreakoutBallLockedIndex",
      "PossessionRules",
      "FireBallObstacle",
      "KeepUpRules",
      "MatchAdmin",
    ],
  },
] as const;

export type MutatorGroupKey = (typeof MUTATOR_GROUPS)[number]["key"];

/** Group of a category ("special" for unknown ones). */
export function groupOf(category: string): MutatorGroupKey {
  for (const g of MUTATOR_GROUPS) {
    if ((g.categories as readonly string[]).includes(category)) return g.key;
  }
  return "special";
}
