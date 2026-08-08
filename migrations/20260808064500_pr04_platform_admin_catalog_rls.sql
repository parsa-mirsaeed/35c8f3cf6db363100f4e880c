-- PR-04: align database RLS with the endpoint authorization manifest.
--
-- PlatformAdmin is the only browser role allowed to manage the global school
-- and subject catalog. Keep the grants deliberately narrower than FOR ALL:
-- school update/delete remain disabled at the API layer, while subject
-- create/update/delete are explicit PlatformAdmin operations.

DROP POLICY IF EXISTS schools_platform_admin_select_policy ON public.schools;
CREATE POLICY schools_platform_admin_select_policy
ON public.schools
FOR SELECT
USING (public.get_role() = 'PlatformAdmin');

DROP POLICY IF EXISTS schools_platform_admin_insert_policy ON public.schools;
CREATE POLICY schools_platform_admin_insert_policy
ON public.schools
FOR INSERT
WITH CHECK (public.get_role() = 'PlatformAdmin');

DROP POLICY IF EXISTS subjects_platform_admin_insert_policy ON public.subjects;
CREATE POLICY subjects_platform_admin_insert_policy
ON public.subjects
FOR INSERT
WITH CHECK (public.get_role() = 'PlatformAdmin');

DROP POLICY IF EXISTS subjects_platform_admin_update_policy ON public.subjects;
CREATE POLICY subjects_platform_admin_update_policy
ON public.subjects
FOR UPDATE
USING (public.get_role() = 'PlatformAdmin')
WITH CHECK (public.get_role() = 'PlatformAdmin');

DROP POLICY IF EXISTS subjects_platform_admin_delete_policy ON public.subjects;
CREATE POLICY subjects_platform_admin_delete_policy
ON public.subjects
FOR DELETE
USING (public.get_role() = 'PlatformAdmin');
