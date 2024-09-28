const { Mf2Extension } = require("./extension.cjs");

const mf2Extension = new Mf2Extension();

exports.activate = mf2Extension.activate.bind(mf2Extension);
exports.deactivate = mf2Extension.deactivate.bind(mf2Extension);
