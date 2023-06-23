// This file was generated by [ts-rs](https://github.com/Aleph-Alpha/ts-rs). Do not edit this file manually.
import type { TeamInvite } from "./TeamInvite";
import type { User } from "./User";

export interface TeamWithMembers { id: number, team_name: string, owner: string, score: number | null, active_bot: number | null, members: Array<User>, invites: Array<TeamInvite> | null, }