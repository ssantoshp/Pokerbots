import React, { useState, useLayoutEffect, useEffect } from "react";
import {
  Team,
  apiUrl,
  pfpEndpoint,
  useMyTeam,
  useTeam,
  useUser,
} from "../state";
import Box from "@mui/system/Box";
import { Container } from "@mui/system";
import AddIcon from "@mui/icons-material/Add";
import TableBody from "@mui/material/TableBody";
import TableContainer from "@mui/material/TableContainer";
import TableRow from "@mui/material/TableRow";
import Table from "@mui/material/Table";
import Avatar from "@mui/material/Avatar";
import CircularProgress from "@mui/material/CircularProgress";
import { primary_background } from "../styles.module.css";
import { BotUpload } from "./BotUpload";
import { TableCell, TableButton } from ".";
import { Button, Icon, TextField, useTheme } from "@mui/material";
import EditIcon from "@mui/icons-material/Edit";
import CopyIcon from "@mui/icons-material/ContentCopy";
import { enqueueSnackbar } from "notistack";

function PfpUpload({ team, readonly }: { team: Team; readonly: boolean }) {
  const [drag, setDrag] = useState(false);
  const fetchTeam = useMyTeam()[1];

  const [boxWidth, setBoxWidth] = useState(0);
  const [uploading, setUploading] = useState(false);

  const handleUpload = async (f: File) => {
    setUploading(true);
    // TODO: Display errors on these api calls
    /*const uploadLink = await (
      await fetch(`${apiUrl}/pfp-upload-url?content_length=${f.size}`)
    ).json();*/
    await fetch(`${apiUrl}/upload-pfp` /*uploadLink.url*/, {
      method: "PUT",
      body: f,
      //headers: uploadLink.headers,
    }).finally(() => {
      setTimeout(() => {
        fetchTeam();
        setUploading(false);
      }, 1000);
    });
  };

  return (
    <Box
      sx={(theme) => ({
        height: "100%",
        minWidth: "10px",

        display: "flex",
        justifyContent: "center",
        alignItems: "center",
        position: "relative",
      })}
    >
      <Avatar
        sx={(theme) => ({
          [theme.breakpoints.down("md")]: {
            width: "100px",
            height: "100px",
          },
          height: `150px`,
          width: `150px`,
          flexDirection: "column",
        })}
        onDragEnter={(e) => {
          if (readonly) return;
          e.preventDefault();
          setDrag(true);
        }}
        onDragOver={(e) => {
          if (readonly) return;
          e.preventDefault();
          setDrag(true);
        }}
        onDragLeave={(e) => {
          if (readonly) return;
          e.preventDefault();
          setDrag(false);
        }}
        onDrop={(e) => {
          if (readonly) return;
          e.preventDefault();
          handleUpload(e.dataTransfer.files[0]);
          setDrag(false);
        }}
        src={
          uploading
            ? ""
            : `${pfpEndpoint}${team?.id}.png?${
                readonly
                  ? ""
                  : Math.floor(
                      Date.now() / 1000
                    ) /* Reset the cache every second */
              }`
        }
      ></Avatar>

      <Box
        sx={{
          position: "absolute",
          color: "white",
          height: `${boxWidth}px`,
          width: `${boxWidth}px`,
          display: "flex",
          justifyContent: "center",
          alignItems: "center",
          borderRadius: "100%",
          pointerEvents: "none",
          transition: "0.2s ease-out",
          ...(drag && {
            backgroundColor: "#66666666",
          }),
        }}
      >
        {drag && "Drop an image here"}
        {uploading && <CircularProgress />}
      </Box>
    </Box>
  );
}

export function TeamBar({ readonly }: { readonly: boolean }) {
  const [team, fetchTeam] = useTeam();
  const [editing, setEditing] = useState(false);
  const theme = useTheme();
  const user = useUser()[0];
  const headerRef = React.useRef<HTMLHeadingElement>(null);
  if (!user || !team)
    throw new Error("Cannot render team bar when not logged in with a team");

  useEffect(() => {
    if (team.team_name) {
      headerRef.current!.textContent = team.team_name;
    }
  }, [team.team_name]);
  return (
    <Box
      className={primary_background}
      sx={{
        color: "white",
        padding: 2,
      }}
    >
      <Container
        sx={{
          flexDirection: "row",
          display: "flex",
          alignItems: "center",
          gap: "16px",
          height: "100%",
        }}
      >
        <PfpUpload team={team} readonly={readonly} />
        <Box
          sx={{
            flexDirection: "column",
          }}
        >
          <Box display="flex" flexDirection="row" alignItems={"baseline"}>
            <h1
              ref={headerRef}
              contentEditable={editing}
              suppressContentEditableWarning={true}
              id={`team-name-${team?.id}-${editing}`}
              style={{
                margin: "10px",
              }}
              onFocus={(e) => {
                window.getSelection()?.selectAllChildren(e.target);
              }}
              onBlur={(e) => {
                setEditing(false);
                fetch(`${apiUrl}/rename-team?to=${e.target.textContent}`).then(
                  async (res) => {
                    const json = await res.json();
                    if (json.error) {
                      enqueueSnackbar(json.error, {
                        variant: "error",
                      });
                      return;
                    }
                    fetchTeam();
                  }
                );
              }}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  const el = e.target as HTMLElement;
                  el.blur();
                }
              }}
            >
              {team?.team_name}
            </h1>
            {readonly || editing || (
              <Box>
                <TableButton
                  sx={{
                    margin: 2,
                  }}
                  onClick={() => {
                    if (!readonly) setEditing(true);
                    // set a timeout so that the focus happens after the contenteditable is enabled
                    setTimeout(() => {
                      if (headerRef.current) {
                        headerRef.current.focus();
                      }
                    }, 5);
                  }}
                >
                  <EditIcon sx={{ mr: "4px" }} fontSize="small" />
                  Rename
                </TableButton>
              </Box>
            )}
          </Box>
          <Box
            sx={{
              flexDirection: "row",
              display: "flex",
              gap: "10px",
            }}
          >
            <Box display="flex">
              <TableContainer>
                <Table size="small">
                  <TableBody>
                    {team.members.map((member) => (
                      <TableRow key={member.email}>
                        <TableCell
                          sx={{
                            color: "white",
                          }}
                        >
                          {member.display_name}
                        </TableCell>
                        {(team.owner == user.email ||
                          member.email == user.email) && (
                          <TableCell
                            sx={{
                              color: "white",
                            }}
                          >
                            {readonly || (
                              <TableButton
                                onClick={() => {
                                  const confirmed = confirm(
                                    `Are you sure you want to ${
                                      member.email == user.email
                                        ? team.owner == user.email
                                          ? "delete the team"
                                          : "leave the team"
                                        : "kick this member"
                                    }?`
                                  );
                                  if (!confirmed) return;

                                  if (member.email == user.email) {
                                    if (team.owner == user.email) {
                                      fetch(`${apiUrl}/delete-team`).then(
                                        (response) => {
                                          fetchTeam();
                                        }
                                      );
                                    } else {
                                      fetch(`${apiUrl}/leave-team`).then(
                                        (response) => {
                                          fetchTeam();
                                        }
                                      );
                                    }
                                  } else {
                                    fetch(
                                      `${apiUrl}/kick-member?email=${member.email}`
                                    ).then((response) => {
                                      fetchTeam();
                                    });
                                  }
                                }}
                              >
                                {member.email == user.email
                                  ? team.owner == user.email
                                    ? "Delete team"
                                    : "Leave"
                                  : "Kick"}
                              </TableButton>
                            )}
                          </TableCell>
                        )}
                      </TableRow>
                    ))}

                    {team.invites &&
                      team.invites.map((invite) => (
                        <TableRow key={invite.invite_code}>
                          <TableCell
                            sx={{
                              color: "white",
                              justifyContent: "center",
                              display: "flex",
                            }}
                          >
                            <input
                              value={`${apiUrl}/join-team?invite_code=${invite.invite_code}`}
                              readOnly
                              style={{
                                backgroundColor: theme.palette.secondary.main,
                                marginRight: "8px",
                              }}
                              onClick={(e) => {
                                const input = e.target as HTMLInputElement;
                                input.select();
                                // modern version of the following command
                                navigator.clipboard.writeText(input.value);
                                enqueueSnackbar("Copied to clipboard", {
                                  variant: "success",
                                });
                              }}
                            />
                            <CopyIcon
                              style={{
                                cursor: "pointer",
                              }}
                              color="secondary"
                              fontSize="small"
                            />
                          </TableCell>
                          {team.owner == user.email && (
                            <TableCell
                              sx={{
                                color: "white",
                              }}
                            >
                              <TableButton
                                onClick={() => {
                                  fetch(
                                    `${apiUrl}/cancel-invite?invite_code=${invite.invite_code}`
                                  ).then(() => fetchTeam());
                                }}
                              >
                                Cancel invitation
                              </TableButton>
                            </TableCell>
                          )}
                        </TableRow>
                      ))}
                    {readonly || (
                      <TableRow>
                        <TableCell
                          sx={{
                            alignItems: "center",
                            justifyContent: "left",
                            display: "flex",
                          }}
                        >
                          <TableButton
                            startIcon={<AddIcon />}
                            onClick={() =>
                              fetch(`${apiUrl}/make-invite`).then(async (a) => {
                                fetchTeam();
                              })
                            }
                          >
                            Add a member
                          </TableButton>
                        </TableCell>
                      </TableRow>
                    )}
                  </TableBody>
                </Table>
              </TableContainer>
            </Box>
          </Box>
          <Box></Box>
        </Box>
      </Container>
    </Box>
  );
}
