import { useWikiStore } from "@/stores/wiki-store"
import { ChatPanel } from "@/components/chat/chat-panel"
import { SettingsView } from "@/components/settings/settings-view"
import { SourcesView } from "@/components/sources/sources-view"
import { ReviewView } from "@/components/review/review-view"
import { LintView } from "@/components/lint/lint-view"
import { SearchView } from "@/components/search/search-view"
import { GraphView } from "@/components/graph/graph-view"
import { WelcomeScreen } from "@/components/project/welcome-screen"
import type { WikiProject } from "@/types/wiki"

interface ContentAreaProps {
  onCreateProject: () => void
  onSelectProject: (project: WikiProject) => void
}

export function ContentArea({ onCreateProject, onSelectProject }: ContentAreaProps) {
  const activeView = useWikiStore((s) => s.activeView)
  const project = useWikiStore((s) => s.project)

  // Without a project, only welcome and settings are available
  if (!project) {
    if (activeView === "settings") return <SettingsView />
    return <WelcomeScreen onCreateProject={onCreateProject} onSelectProject={onSelectProject} />
  }

  switch (activeView) {
    case "settings":
      return <SettingsView />
    case "sources":
      return <SourcesView />
    case "review":
      return <ReviewView />
    case "lint":
      return <LintView />
    case "search":
      return <SearchView />
    case "graph":
      return <GraphView />
    default:
      return <ChatPanel />
  }
}
