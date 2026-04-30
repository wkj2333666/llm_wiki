import { useEffect, useState } from "react"
import { Plus, FolderOpen, Loader2 } from "lucide-react"
import { Button } from "@/components/ui/button"
import { listProjects, openProject } from "@/commands/fs"
import type { WikiProject } from "@/types/wiki"
import { useTranslation } from "react-i18next"

interface WelcomeScreenProps {
  onCreateProject: () => void
  onSelectProject: (project: WikiProject) => void
}

export function WelcomeScreen({
  onCreateProject,
  onSelectProject,
}: WelcomeScreenProps) {
  const { t } = useTranslation()
  const [projects, setProjects] = useState<{ name: string; path: string; has_wiki: boolean }[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [opening, setOpening] = useState<string | null>(null)

  useEffect(() => {
    loadProjects()
  }, [])

  async function loadProjects() {
    setLoading(true)
    setError(null)
    try {
      const list = await listProjects()
      setProjects(list)
    } catch (err) {
      setError(String(err))
    } finally {
      setLoading(false)
    }
  }

  async function handleOpenProject(name: string) {
    setOpening(name)
    setError(null)
    try {
      const project = await openProject(name)
      onSelectProject(project)
    } catch (err) {
      setError(String(err))
    } finally {
      setOpening(null)
    }
  }

  return (
    <div className="flex h-full items-center justify-center bg-background">
      <div className="flex flex-col items-center gap-8 px-4">
        <div className="text-center">
          <h1 className="text-3xl font-bold">{t("app.title")}</h1>
          <p className="mt-2 text-muted-foreground">
            {t("app.subtitle")}
          </p>
        </div>

        <Button onClick={onCreateProject}>
          <Plus className="mr-2 h-4 w-4" />
          {t("welcome.newProject")}
        </Button>

        {error && (
          <p className="text-sm text-destructive">{error}</p>
        )}

        <div className="w-full max-w-md">
          <div className="mb-2 flex items-center gap-2 text-sm text-muted-foreground">
            <FolderOpen className="h-3.5 w-3.5" />
            {loading ? t("welcome.loadingProjects", "Loading...") : t("welcome.availableProjects", "Available Projects")}
          </div>

          {loading ? (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
            </div>
          ) : projects.length === 0 ? (
            <div className="rounded-lg border p-4 text-center text-sm text-muted-foreground">
              {t("welcome.noProjects", "No projects found. Create a new project to start.")}
            </div>
          ) : (
            <div className="rounded-lg border">
              {projects.map((proj) => (
                <button
                  key={proj.name}
                  onClick={() => handleOpenProject(proj.name)}
                  disabled={opening === proj.name || !proj.has_wiki}
                  className="group flex w-full items-center justify-between border-b px-4 py-3 text-left transition-colors last:border-b-0 hover:bg-accent disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-sm font-medium">{proj.name}</div>
                    <div className="truncate text-xs text-muted-foreground">
                      {!proj.has_wiki && (
                        <span className="text-amber-500">{t("welcome.invalidProject", "Not a wiki project")} — </span>
                      )}
                      {proj.path}
                    </div>
                  </div>
                  <div className="ml-2 shrink-0">
                    {opening === proj.name ? (
                      <Loader2 className="h-4 w-4 animate-spin" />
                    ) : (
                      <FolderOpen className="h-4 w-4 text-muted-foreground opacity-0 group-hover:opacity-100 transition-opacity" />
                    )}
                  </div>
                </button>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  )
}