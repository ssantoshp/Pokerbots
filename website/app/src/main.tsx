import * as React from "react";
import * as ReactDOM from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import "./styles.module.css";
import { SnackbarProvider } from "notistack";

//import "../static/css/styles.css";
import PokerZero from "./PokerZero";
import { ThemeProvider } from "@mui/material";
import theme from "./theme";

function RootApp() {
  return (
    <SnackbarProvider maxSnack={3}>
      <ThemeProvider theme={theme}>
        <BrowserRouter>
          <PokerZero />
        </BrowserRouter>
      </ThemeProvider>
    </SnackbarProvider>
  );
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <RootApp />
);
