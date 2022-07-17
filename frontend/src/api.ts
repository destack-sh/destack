import axios from "axios";
import qs from "qs";

export const api = axios.create({
  baseURL: "http://localhost:8000/api/",
  paramsSerializer: (params) => {
    return qs.stringify(params, { arrayFormat: "repeat" });
  },
});
