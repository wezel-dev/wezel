import { useContext } from "react";
import { ProjectCtx } from "./projectCtx";

export function useProject() {
  return useContext(ProjectCtx);
}
