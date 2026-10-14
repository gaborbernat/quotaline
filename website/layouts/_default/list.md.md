# {{ .Title }}
{{ with .Description }}
> {{ . }}
{{ end }}
{{ .RawContent }}
{{ range .Pages.ByWeight }}
- [{{ .Title }}]({{ .RelPermalink }}){{ with .Description }}: {{ . }}{{ end }}
{{ end }}

---

{{ .Site.Title }}: {{ .Site.Params.repository }}
