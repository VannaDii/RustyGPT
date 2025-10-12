-- Health check stored procedure expected by readiness probe
CREATE OR REPLACE PROCEDURE rustygpt.sp_healthz()
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM 1;
END;
$$;
