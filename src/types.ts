import { z } from "zod";

// ProblemStats Schema
export const ProblemStatsSchema = z.object({
  solved: z.boolean(),
  attempted_during_freeze: z.boolean(),
  penalty: z.number(),
  submissions_before_solved: z.number(),
  first_ac_time: z.iso
    .datetime({ offset: true })
    .transform((str) => new Date(str))
    .nullable(),
});

// TeamStats Schema
export const TeamStatsSchema = z.object({
  team_id: z.string(),
  team_name: z.string(),
  team_affiliation: z.string(),
  sortorder: z.number(),
  total_points: z.number(),
  total_penalty: z.number(),
  last_ac_time: z.iso
    .datetime({ offset: true })
    .transform((str) => new Date(str))
    .nullable(),
  problem_stats: z.record(z.string(), ProblemStatsSchema),
});

export const ProblemSchema = z.object({
  ordinal: z.number().int().nonnegative(),
  id: z.string().min(1),
  short_name: z.string().min(1),
  rgb: z.string().regex(/^#[0-9A-F]{6}$/i),
  color: z.string().min(1),
  label: z.string().min(1),
  time_limit: z.number().positive(),
  statement: z.array(z.any()),
  externalid: z.string().nullable(),
  name: z.string().min(1),
  test_data_count: z.number().int().nonnegative(),
});

export const AwardSchema = z.object({
  id: z.string(),
  citation: z.string(),
  team_ids: z.array(z.string()),
});

export const ParsedRoot = z.object({
  scoreboard: z.array(TeamStatsSchema),
  problems: z.array(ProblemSchema),
  awards: z.record(z.string(), z.array(z.string())),
});

export type Problem = z.infer<typeof ProblemSchema>;
export type ProblemStats = z.infer<typeof ProblemStatsSchema>;
export type TeamStats = z.infer<typeof TeamStatsSchema>;
export type Award = z.infer<typeof AwardSchema>;
export type ParsedRoot = z.infer<typeof ParsedRoot>;
