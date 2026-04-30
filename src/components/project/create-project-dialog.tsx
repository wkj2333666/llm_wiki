import { useState } from "react"
import { useTranslation } from "react-i18next"
import {
  Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter,
} from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { createProject, writeFile, createDirectory } from "@/commands/fs"
import { getTemplate } from "@/lib/templates"
import { TemplatePicker } from "@/components/project/template-picker"
import type { WikiProject } from "@/types/wiki"
import { normalizePath } from "@/lib/path-utils"
import { OUTPUT_LANGUAGE_OPTIONS } from "@/lib/output-language-options"
import { useWikiStore, type OutputLanguage } from "@/stores/wiki-store"
import { saveOutputLanguage } from "@/lib/project-store"

interface CreateProjectDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  onCreated: (project: WikiProject) => void
}

export function CreateProjectDialog({ open: isOpen, onOpenChange, onCreated }: CreateProjectDialogProps) {
  const { t } = useTranslation()
  const [name, setName] = useState("")
  const [selectedTemplate, setSelectedTemplate] = useState("general")
  const [language, setLanguage] = useState<string>("Chinese")  // 默认简体中文
  const [error, setError] = useState("")
  const [creating, setCreating] = useState(false)
  const setOutputLanguage = useWikiStore((s) => s.setOutputLanguage)

  async function handleCreate() {
    if (!name.trim()) {
      setError(t("project.nameRequired", "请输入项目名称"))
      return
    }
    setCreating(true)
    setError("")
    try {
      // Create project - server uses configured projects_dir
      const project = await createProject(name.trim())
      const pp = normalizePath(project.path)

      const template = getTemplate(selectedTemplate)
      await writeFile(`${pp}/schema.md`, template.schema)
      await writeFile(`${pp}/purpose.md`, template.purpose)
      for (const dir of template.extraDirs) {
        await createDirectory(`${pp}/${dir}`)
      }

      const lang = language as OutputLanguage
      setOutputLanguage(lang)
      await saveOutputLanguage(lang)

      onCreated(project)
      onOpenChange(false)
      setName("")
      setSelectedTemplate("general")
      setLanguage("Chinese")
    } catch (err) {
      setError(String(err))
    } finally {
      setCreating(false)
    }
  }

  return (
    <Dialog open={isOpen} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>{t("project.createTitle", "新建 Wiki 项目")}</DialogTitle>
        </DialogHeader>
        <div className="flex flex-col gap-4 py-4">
          <div className="flex flex-col gap-2">
            <Label htmlFor="name">{t("project.name", "项目名称")}</Label>
            <Input id="name" value={name} onChange={(e) => setName(e.target.value)} placeholder={t("project.namePlaceholder", "my-research-wiki")} />
            <p className="text-xs text-muted-foreground">
              {t("project.serverDirHint", "项目将创建在服务器配置的目录下")}
            </p>
          </div>
          <div className="flex flex-col gap-2">
            <Label>{t("project.template", "模板")}</Label>
            <TemplatePicker selected={selectedTemplate} onSelect={setSelectedTemplate} />
          </div>
          <div className="flex flex-col gap-2">
            <Label htmlFor="language">
              {t("settings.sections.output.aiLanguage", "AI 输出语言")}
            </Label>
            <select
              id="language"
              value={language}
              onChange={(e) => setLanguage(e.target.value)}
              className="w-full rounded-md border bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
            >
              {OUTPUT_LANGUAGE_OPTIONS.filter((l) => l.value !== "auto").map((l) => (
                <option key={l.value} value={l.value}>
                  {l.label}
                </option>
              ))}
            </select>
            <p className="text-xs text-muted-foreground">
              {t("settings.sections.output.aiLanguageHint", "所有 AI 生成内容将使用此语言。可在设置中更改。")}
            </p>
          </div>
          {error && <p className="text-sm text-destructive">{error}</p>}
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>{t("project.cancel", "取消")}</Button>
          <Button onClick={handleCreate} disabled={creating}>{creating ? t("project.creating", "创建中...") : t("project.create", "创建")}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}