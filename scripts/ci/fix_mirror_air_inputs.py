from pathlib import Path

mirror_path = Path('.github/workflows/mirror-final-proof.yml')
mirror = mirror_path.read_text(encoding='utf-8')
old = '''    with:
      complete: true
      publish: false
      create_archive: false
'''
new = '''    with:
      complete: true
      create_archive: false
'''
if mirror.count(old) != 1:
    raise SystemExit('expected stale mirror appliance input block exactly once')
mirror_path.write_text(mirror.replace(old, new), encoding='utf-8')

validate_path = Path('scripts/appliance/validate.sh')
validate = validate_path.read_text(encoding='utf-8')
anchor = '''grep -Fq 'complete: true' "${mirror_workflow}"
if grep -Fq "gh workflow run air-gapped-appliance.yml" "${mirror_workflow}"; then
'''
replacement = '''grep -Fq 'complete: true' "${mirror_workflow}"
if grep -Fq 'publish: false' "${mirror_workflow}"; then
  echo "Mirror passed an unsupported publication input to the read-only appliance proof." >&2
  exit 1
fi
if grep -Fq "gh workflow run air-gapped-appliance.yml" "${mirror_workflow}"; then
'''
if validate.count(anchor) != 1:
    raise SystemExit('expected mirror input validation anchor exactly once')
validate_path.write_text(validate.replace(anchor, replacement), encoding='utf-8')
