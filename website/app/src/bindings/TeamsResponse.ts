// This file was generated by [ts-rs](https://github.com/Aleph-Alpha/ts-rs). Do not edit this file manually.
import type { Team } from "./Team";
import type { TeamWithMembers } from "./TeamWithMembers";

export type TeamsResponse = { Count: bigint } | { Teams: Array<Team> } | { TeamsWithMembers: Array<TeamWithMembers> };