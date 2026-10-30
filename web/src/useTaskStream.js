import { useEffect, useState } from "react";
import { observeTask } from "./transport.js";

export function useTaskStream(job, token, navigation) {
  const [overview, setOverview] = useState(null);
  const [detail, setDetail] = useState(null);
  const [connection, setConnection] = useState("connecting");
  const [error, setError] = useState("");
  useEffect(
    () =>
      observeTask({
        job,
        token,
        onSample: (value) => {
          setOverview(value.overview);
          setDetail(value.detail || null);
        },
        onConnection: setConnection,
        onError: setError,
      }),
    [job, token, navigation],
  );
  return {
    overview,
    detail,
    setDetail,
    connection,
    error,
    clearError: () => setError(""),
  };
}
