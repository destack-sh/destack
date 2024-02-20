module.exports = {
  client: {
    service: {
      name: "bench",
      url: "http://localhost:8000/graphql",
    },
    includes: ["bench-web/src/**/*.vue", "bench-web/src/**/*.js", "bench-web/src/**/*.ts"],
  },
};
