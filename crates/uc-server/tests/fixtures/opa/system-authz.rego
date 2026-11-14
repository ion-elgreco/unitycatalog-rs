# OPA's own access rules, used with `opa run --authentication=token
# --authorization=basic`. The catalog server's token may only ask for
# decisions of uc.authz, the admin token may do everything, and health
# checks need no token. The tokens come from OPA's environment.

package system.authz

default allow := false

allow if {
	input.identity == opa.runtime().env.OPA_CATALOG_TOKEN
	input.method == "POST"
	input.path == ["v1", "data", "uc", "authz"]
}

allow if input.identity == opa.runtime().env.OPA_ADMIN_TOKEN

allow if input.path == ["health"]
