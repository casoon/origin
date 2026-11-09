import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

test("app shell renders", async ({ page }) => {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));

  await page.goto("/");

  await expect(page).toHaveTitle("__PRODUCT_NAME__");
  await expect(page.getByRole("heading", { level: 1 })).toHaveText("__PRODUCT_NAME__");
  await expect(page.getByRole("main")).toBeVisible();

  expect(pageErrors).toEqual([]);
});

test("app shell has no accessibility violations", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("main")).toBeVisible();

  const results = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag22aa"])
    .analyze();

  expect(results.violations).toEqual([]);
});
