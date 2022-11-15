module.exports = {
  client: {
    service: {
      name: "bench",
      url: "http://localhost:8000/graphql",
    },
    includes: ["frontend/src/**/*.vue", "frontend/src/**/*.js", "frontend/src/**/*.ts"],
  },
};
