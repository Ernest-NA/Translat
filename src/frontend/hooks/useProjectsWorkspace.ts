import { useCallback, useEffect, useMemo, useState } from "react";
import type {
  CreateProjectInput,
  ProjectSummary,
  ProjectsOverview,
} from "../../shared/desktop";
import {
  createProject,
  DesktopCommandError,
  listProjects,
  openProject,
} from "../lib/desktop";

function sortProjects(projects: ProjectSummary[]) {
  return [...projects].sort((left, right) => {
    if (left.lastOpenedAt !== right.lastOpenedAt) {
      return right.lastOpenedAt - left.lastOpenedAt;
    }

    if (left.createdAt !== right.createdAt) {
      return right.createdAt - left.createdAt;
    }

    return left.name.localeCompare(right.name, undefined, {
      sensitivity: "base",
    });
  });
}

function normalizeProjectsOverview(
  overview: ProjectsOverview,
): ProjectsOverview {
  return {
    activeProjectId: overview.activeProjectId,
    projects: sortProjects(overview.projects),
  };
}

export function useProjectsWorkspace() {
  const [overview, setOverview] = useState<ProjectsOverview>({
    activeProjectId: null,
    projects: [],
  });
  const [error, setError] = useState<DesktopCommandError | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isCreating, setIsCreating] = useState(false);
  const [openingProjectId, setOpeningProjectId] = useState<string | null>(null);

  const reload = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    try {
      const nextOverview = await listProjects();
      setOverview(normalizeProjectsOverview(nextOverview));
    } catch (caughtError) {
      setError(
        caughtError instanceof DesktopCommandError
          ? caughtError
          : new DesktopCommandError("list_projects", {
              code: "UNEXPECTED_DESKTOP_ERROR",
              message: "The desktop shell returned an unknown project error.",
            }),
      );
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  const submitProject = useCallback(
    async (input: CreateProjectInput): Promise<boolean> => {
      setIsCreating(true);
      setError(null);

      try {
        const createdProject = await createProject(input);
        setOverview((currentOverview) =>
          normalizeProjectsOverview({
            activeProjectId: createdProject.id,
            projects: [
              createdProject,
              ...currentOverview.projects.filter(
                (project) => project.id !== createdProject.id,
              ),
            ],
          }),
        );
        return true;
      } catch (caughtError) {
        setError(
          caughtError instanceof DesktopCommandError
            ? caughtError
            : new DesktopCommandError("create_project", {
                code: "UNEXPECTED_DESKTOP_ERROR",
                message: "The desktop shell could not create the project.",
              }),
        );
        return false;
      } finally {
        setIsCreating(false);
      }
    },
    [],
  );

  const selectProject = useCallback(
    async (projectId: string): Promise<boolean> => {
      setOpeningProjectId(projectId);
      setError(null);

      try {
        const openedProject = await openProject({ projectId });
        setOverview((currentOverview) =>
          normalizeProjectsOverview({
            activeProjectId: openedProject.id,
            projects: [
              openedProject,
              ...currentOverview.projects.filter(
                (project) => project.id !== openedProject.id,
              ),
            ],
          }),
        );
        return true;
      } catch (caughtError) {
        setError(
          caughtError instanceof DesktopCommandError
            ? caughtError
            : new DesktopCommandError("open_project", {
                code: "UNEXPECTED_DESKTOP_ERROR",
                message: "The desktop shell could not open the project.",
              }),
        );
        return false;
      } finally {
        setOpeningProjectId(null);
      }
    },
    [],
  );

  const activeProject = useMemo(
    () =>
      overview.projects.find(
        (project) => project.id === overview.activeProjectId,
      ) ?? null,
    [overview.activeProjectId, overview.projects],
  );

  return {
    activeProject,
    error,
    isCreating,
    isLoading,
    openingProjectId,
    projects: overview.projects,
    reload,
    selectProject,
    submitProject,
  };
}
