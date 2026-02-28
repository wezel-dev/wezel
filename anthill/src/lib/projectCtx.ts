import { createContext } from "react";
import type { ProjectApi } from "./api";
import type { Project } from "./data";

export interface ProjectCtxValue {
  projects: Project[];
  current: Project | null;
  setCurrent: (p: Project) => void;
  pApi: ProjectApi;
}

/** Fallback api that always rejects — used before a project is selected. */
export const nullApi: ProjectApi = {
  overview: () => Promise.reject("no project"),
  scenarios: () => Promise.reject("no project"),
  scenario: () => Promise.reject("no project"),
  togglePin: () => Promise.reject("no project"),
  commits: () => Promise.reject("no project"),
  commit: () => Promise.reject("no project"),
  users: () => Promise.reject("no project"),
};

export const ProjectCtx = createContext<ProjectCtxValue>({
  projects: [],
  current: null,
  setCurrent: () => {},
  pApi: nullApi,
});