import type * as lsp from "vscode-languageserver-protocol";

type RequestNames = (keyof typeof lsp) & `${string}Request`;

type Requests = {
  [Name in RequestNames as (typeof lsp)[Name]["method"]]: NonNullable<
    (typeof lsp)[Name]["type"]["_"]
  >;
};

export type RequestMethods = keyof Requests;
export type RequestParams<T extends RequestMethods> = Requests[T][0];
export type RequestResponse<T extends RequestMethods> = Requests[T][1];

type UnfilteredNotificationNames =
  & (keyof typeof lsp)
  & `${string}Notification`;
type NotificationNames<T = UnfilteredNotificationNames> = T extends
  infer U extends UnfilteredNotificationNames
  ? (typeof lsp)[U] extends { method: string } ? U : never
  : never;

type Notifications = {
  [Name in NotificationNames as (typeof lsp)[Name]["method"]]: NonNullable<
    (typeof lsp)[Name]["type"]["_"]
  >[0];
};

export type NotificationMethods = keyof Notifications;
export type NotificationParams<T extends NotificationMethods> =
  Notifications[T] extends lsp._EM ? undefined
    : Notifications[T];
