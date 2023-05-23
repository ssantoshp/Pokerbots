import React from "react";
import { Route, Routes, useNavigate, useParams } from "react-router-dom";
import HomePage from "./HomePage";
import ManageTeam from "./ManageTeam";
import { Team, useMyTeam, useTeam, useUser } from "./state";

import logoImage from "../static/assets/logo.webp";
import { TopBar, BottomBar } from "./components/AppBar";
import { Box, Container } from "@mui/system";
import Leaderboard from "./Leaderboard";
import { primary_background } from "./styles.module.css";
import { CircularProgress, LinearProgress } from "@mui/material";
import { useAtom } from "jotai";

function HeaderFooter(props: React.PropsWithChildren<{}>) {
  const user = useUser()[0];
  const navigate = useNavigate();

  return (
    <Box
      display={"flex"}
      flexDirection={"column"}
      height="100%"
      className={primary_background}
    >
      <TopBar />
      {props.children}

      <BottomBar />
    </Box>
  );
}

function TeamDashboard() {
  const params = useParams();

  const [myTeam] = useMyTeam();
  const [team, fetchTeam, setTeam] = useTeam();

  //TODO: Use suspense here
  React.useEffect(() => {
    setTeam(params.teamId);
  }, [params.teamId]);
  return <ManageTeam readonly={team?.id !== myTeam?.id} />;
}

export default function PokerZero() {
  return (
    <Routes>
      <Route path="/">
        <Route
          index
          element={
            <HeaderFooter>
              <HomePage />
            </HeaderFooter>
          }
        />
        <Route
          path="manage-team"
          element={
            <HeaderFooter>
              <TeamDashboard />
            </HeaderFooter>
          }
        />
        <Route
          path="leaderboard"
          element={
            <HeaderFooter>
              <Leaderboard />
            </HeaderFooter>
          }
        />
        <Route path="team">
          <Route
            path=":teamId"
            element={
              <HeaderFooter>
                <TeamDashboard />
              </HeaderFooter>
            }
          />
        </Route>
      </Route>
    </Routes>
  );
}