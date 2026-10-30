{{- define "unitycatalog.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{- define "unitycatalog.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{- define "unitycatalog.server.fullname" -}}
{{- printf "%s-server" (include "unitycatalog.fullname" .) | trunc 63 | trimSuffix "-" }}
{{- end }}

{{- define "unitycatalog.labels" -}}
helm.sh/chart: {{ printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}

{{- define "unitycatalog.server.selectorLabels" -}}
app.kubernetes.io/name: {{ include "unitycatalog.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
app.kubernetes.io/component: server
{{- end }}

{{- define "unitycatalog.server.labels" -}}
{{ include "unitycatalog.labels" . }}
{{ include "unitycatalog.server.selectorLabels" . }}
{{- end }}

{{- define "unitycatalog.serviceAccountName" -}}
{{- if .Values.serviceAccount.create }}
{{- default (include "unitycatalog.fullname" .) .Values.serviceAccount.name }}
{{- else }}
{{- default "default" .Values.serviceAccount.name }}
{{- end }}
{{- end }}

{{- define "unitycatalog.server.keysSecretName" -}}
{{- default (printf "%s-keys" (include "unitycatalog.server.fullname" .)) .Values.server.keysSecret.name }}
{{- end }}

{{/*
The server's uc.toml: server.config.toml, then the users of auth.users,
which the server makes at each start when they are missing, then the
identity providers of auth.issuers.
*/}}
{{- define "unitycatalog.server.ucToml" -}}
{{ .Values.server.config.toml }}
{{- range $index, $user := .Values.auth.users }}

[[auth.users]]
email = {{ required (printf "auth.users[%d].email is required" $index) $user.email | toJson }}
{{- with $user.name }}
name = {{ . | toJson }}
{{- end }}
{{- with $user.externalId }}
external_id = {{ . | toJson }}
{{- end }}
{{- with $user.privileges }}
privileges = {{ . | toJson }}
{{- end }}
{{- end }}
{{- range $index, $issuer := .Values.auth.issuers }}

[[auth.issuers]]
name = {{ required (printf "auth.issuers[%d].name is required" $index) $issuer.name | toJson }}
issuer = {{ required (printf "auth.issuers[%d].issuer is required" $index) $issuer.issuer | toJson }}
audiences = {{ required (printf "auth.issuers[%d].audiences is required" $index) $issuer.audiences | toJson }}
{{- if $issuer.direct }}
direct = true
{{- end }}
{{- with $issuer.principalClaims }}
principal_claims = {{ . | toJson }}
{{- end }}
{{- with $issuer.clientClaims }}
client_claims = {{ . | toJson }}
{{- end }}
{{- with $issuer.groupsClaim }}
groups_claim = {{ . | toJson }}
{{- end }}
{{- if $issuer.createUsers }}
create_users = true
{{- end }}
{{- with $issuer.displayName }}
display_name = {{ . | toJson }}
{{- end }}
{{- end }}
{{- end }}

{{/*
The PostgreSQL URL (`UC_DB_URL`) of `server.db.postgresqlConfig`.
*/}}
{{- define "unitycatalog.server.db.postgresqlUrl" -}}
{{- $c := .Values.server.db.postgresqlConfig -}}
{{- $params := dict -}}
{{- if $c.ssl.enabled -}}
{{- $_ := set $params "sslmode" $c.ssl.mode -}}
{{- if or $c.ssl.rootCert $c.ssl.rootCertConfigMapName -}}
{{- $_ := set $params "sslrootcert" "/etc/uc/config/dbrootca.crt" -}}
{{- end -}}
{{- end -}}
{{- range $k, $v := $c.extraParams -}}
{{- $_ := set $params $k (toString $v) -}}
{{- end -}}
{{- $query := list -}}
{{- range $k := keys $params | sortAlpha -}}
{{- $query = append $query (printf "%s=%s" $k (get $params $k | urlquery)) -}}
{{- end -}}
postgresql://{{ required "server.db.postgresqlConfig.host is required" $c.host }}:{{ $c.port }}/{{ $c.dbName }}{{ if $query }}?{{ join "&" $query }}{{ end }}
{{- end }}

{{- define "unitycatalog.server.db.filePvcName" -}}
{{- default (printf "%s-db" (include "unitycatalog.server.fullname" .)) .Values.server.db.fileConfig.persistence.pvcName }}
{{- end }}
