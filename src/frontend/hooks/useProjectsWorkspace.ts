import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type {
  CreateProjectInput,
  ProjectSummary,
  ProjectsOverview,
  UpdateProjectEditorialDefaultsInput,
} from "../../shared/desktop";
import {
  createProject,
  DesktopCommandError,
  listProjects,
  openProject,
  updateProjectEditorialDefaults,
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

function buildUnexpectedError(
  command: string,
  message: string,
): DesktopCommandError {
  return new DesktopCommandError(command as never, {
    code: "UNEXPECTED_DESKTOP_ERROR",
    message,
  });
}

export function useProjectsWorkspace() {
  const [overview, setOverview] = useState<ProjectsOverview>({
    activeProjectId: null,
    projects: [],
  });
  const [error, setError] = useState<DesktopCommandError | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isCreating, setIsCreating] = useState(false);
  const [isSavingEditorialDefaults, setIsSavingEditorialDefaults] =
    useState(false);
  const [openingProjectId, setOpeningProjectId] = useState<string | null>(null);
  const latestReloadRequestRef = useRef(0);
  const localStateVersionRef = useRef(0);
  const selectionIntentVersionRef = useRef(0);
  const latestOpenRequestVersionRef = useRef(0);

  const applyLocalOverview = useCallback(
    (
      updateOverview: (currentOverview: ProjectsOverview) => ProjectsOverview,
    ) => {
      localStateVersionRef.current += 1;
      setOverview((currentOverview) =>
        normalizeProjectsOverview(updateOverview(currentOverview)),
      );
    },
    [],
  );

  const reload = useCallback(async () => {
    const reloadRequestId = latestReloadRequestRef.current + 1;
    const localStateVersionAtStart = localStateVersionRef.current;

    latestReloadRequestRef.current = reloadRequestId;
    setIsLoading(true);
    setError(null);

    try {
      const nextOverview = await listProjects();

      if (reloadRequestId !== latestReloadRequestRef.current) {
        return;
      }

      if (localStateVersionAtStart !== localStateVersionRef.current) {
        return;
      }

      setOverview(normalizeProjectsOverview(nextOverview));
    } catch (caughtError) {
      if (reloadRequestId === latestReloadRequestRef.current) {
        setError(
          caughtError instanceof DesktopCommandError
            ? caughtError
            : buildUnexpectedError(
                "list_projects",
                "The desktop shell returned an unknown project error.",
              ),
        );
      }
    } finally {
      if (reloadRequestId === latestReloadRequestRef.current) {
        setIsLoading(false);
      }
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  const submitProject = useCallback(
    async (input: CreateProjectInput): Promise<boolean> => {
      const selectionIntentVersion = selectionIntentVersionRef.current + 1;

      selectionIntentVersionRef.current = selectionIntentVersion;
      setIsCreating(true);
      setError(null);

      try {
        const createdProject = await createProject(input);
        applyLocalOverview((currentOverview) => ({
          activeProjectId:
            selectionIntentVersion === selectionIntentVersionRef.current
              ? createdProject.id
              : currentOverview.activeProjectId,
          projects: [
            createdProject,
            ...currentOverview.projects.filter(
              (project) => project.id !== createdProject.id,
            ),
          ],
        }));
        return true;
      } catch (caughtError) {
        if (selectionIntentVersion === selectionIntentVersionRef.current) {
          setError(
            caughtError instanceof DesktopCommandError
              ? caughtError
              : buildUnexpectedError(
                  "create_project",
                  "The desktop shell could not create the project.",
                ),
          );
        }
        return false;
      } finally {
        setIsCreating(false);
      }
    },
    [applyLocalOverview],
  );

  const selectProject = useCallback(
    async (projectId: string): Promise<boolean> => {
      const selectionIntentVersion = selectionIntentVersionRef.current + 1;

      selectionIntentVersionRef.current = selectionIntentVersion;
      latestOpenRequestVersionRef.current = selectionIntentVersion;
      setOpeningProjectId(projectId);
      setError(null);

      try {
        const openedProject = await openProject({ projectId });
        applyLocalOverview((currentOverview) => ({
          activeProjectId:
            selectionIntentVersion === selectionIntentVersionRef.current
              ? openedProject.id
              : currentOverview.activeProjectId,
          projects: [
            openedProject,
            ...currentOverview.projects.filter(
              (project) => project.id !== openedProject.id,
            ),
          ],
        }));
        return true;
      } catch (caughtError) {
        if (selectionIntentVersion === selectionIntentVersionRef.current) {
          setError(
            caughtError instanceof DesktopCommandError
              ? caughtError
              : buildUnexpectedError(
                  "open_project",
                  "The desktop shell could not open the project.",
                ),
          );
        }
        return false;
      } finally {
        if (selectionIntentVersion === latestOpenRequestVersionRef.current) {
          setOpeningProjectId(null);
        }
      }
    },
    [applyLocalOverview],
  );

  const saveProjectEditorialDefaults = useCallback(
    async (input: UpdateProjectEditorialDefaultsInput): Promise<boolean> => {
      const selectionIntentVersion = selectionIntentVersionRef.current + 1;

      selectionIntentVersionRef.current = selectionIntentVersion;
      setIsSavingEditorialDefaults(true);
      setError(null);

      try {
        const updatedProject = await updateProjectEditorialDefaults(input);
        applyLocalOverview((currentOverview) => ({
          activeProjectId:
            selectionIntentVersion === selectionIntentVersionRef.current
              ? updatedProject.id
              : currentOverview.activeProjectId,
          projects: currentOverview.projects.map((project) =>
            project.id === updatedProject.id ? updatedProject : project,
          ),
        }));
        return true;
      } catch (caughtError) {
        if (selectionIntentVersion === selectionIntentVersionRef.current) {
          setError(
            caughtError instanceof DesktopCommandError
              ? caughtError
              : buildUnexpectedError(
                  "update_project_editorial_defaults",
                  "The desktop shell could not save the project editorial defaults.",
                ),
          );
        }
        return false;
      } finally {
        setIsSavingEditorialDefaults(false);
      }
    },
    [applyLocalOverview],
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
    isSavingEditorialDefaults,
    openingProjectId,
    projects: overview.projects,
    reload,
    saveProjectEditorialDefaults,
    selectProject,
    submitProject,
  };
}
