// This file was generated by [ts-rs](https://github.com/Aleph-Alpha/ts-rs). Do not edit this file manually.
import type { BuildStatus } from "./BuildStatus";

export interface BotWithTeam<T> { id: number, team: T, name: string, description: string | null, score: number, created: bigint, uploaded_by: string, build_status: BuildStatus, }