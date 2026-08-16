# Administrator guide — enabled scope only

This guide covers **PlatformAdmin** and **SchoolManager** capabilities that are currently enabled. It deliberately omits disabled reporting, attendance, timetable and unsupported user-mutation workflows.

## PlatformAdmin

Supported activities include authenticated platform administration for schools, supported subject/platform records, and governed knowledge-asset administration. Authorization is platform-scoped and server enforced.

Do not treat direct IDs as authorization. Do not share operator/database/provider credentials with platform users.

## SchoolManager

Supported activities include school-scoped class/enrollment views and mutations exposed by the current UI/API, supported school-user views/statistics, profile-change request decisions, settings/preferences, and manager knowledge submissions.

A SchoolManager must not access another school's objects. Public signup is disabled. Only workflows shown as Implemented in `feature-matrix.md` may be represented as available.

## Not available

Attendance workflow, timetable management, school-manager reports, derived academic metrics, parent/teacher messaging and any endpoint family explicitly marked `Disabled` are not part of administrator scope.
