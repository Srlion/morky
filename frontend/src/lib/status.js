/** @enum {string} */
export const AppStatus = /** @type {const} */ ({
  IDLE: "idle",
  DEPLOYING: "deploying",
  RUNNING: "running",
  FAILED: "failed",
});

/** @enum {string} */
export const DeployStatus = /** @type {const} */ ({
  BUILDING: "building",
  DEPLOYING: "deploying",
  DONE: "done",
  FAILED: "failed",
  CANCELLED: "cancelled",
});
